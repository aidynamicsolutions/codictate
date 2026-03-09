//! Filesystem and archive utility helpers for backup/restore.
//!
//! Provides path resolution, safe copy/extract, atomic writes, fsync helpers,
//! and shared checksum utilities used by backup/preflight/restore flows.

use super::*;
pub(super) fn app_data_dir<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))
}

pub(super) fn runtime_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(BACKUP_RUNTIME_DIR)
}

pub(super) fn marker_path(app_data_dir: &Path) -> PathBuf {
    runtime_dir(app_data_dir).join(MARKER_FILE_NAME)
}

pub(super) fn undo_checkpoint_meta_path(app_data_dir: &Path) -> PathBuf {
    runtime_dir(app_data_dir).join(UNDO_CHECKPOINT_META_FILE_NAME)
}

#[cfg(not(test))]
pub(super) fn reload_user_store_state<R: tauri::Runtime>(app: &AppHandle<R>) {
    match app.store(USER_STORE_DB_FILE) {
        Ok(store) => {
            if let Err(error) = store.reload_ignore_defaults() {
                warn!(
                    error = %error,
                    "Failed to reload in-memory user store after backup/restore swap"
                );
            }
        }
        Err(error) => {
            warn!(
                error = %error,
                "Failed to access user store for post-restore reload"
            );
        }
    }
}

#[cfg(test)]
pub(super) fn reload_user_store_state<R: tauri::Runtime>(_app: &AppHandle<R>) {}

pub(super) fn write_json_file_atomically<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create JSON payload parent directory: {error}"))?;
    }

    let tmp_name = format!(
        ".tmp-{}-{}",
        std::process::id(),
        timestamp_millis()
    );
    let tmp_path = path.with_file_name(tmp_name);

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp_path)
        .map_err(|error| format!("Failed to create temporary JSON file: {error}"))?;

    serde_json::to_writer_pretty(&mut file, value)
        .map_err(|error| format!("Failed to serialize JSON payload: {error}"))?;

    file.flush()
        .map_err(|error| format!("Failed to flush temporary JSON file: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Failed to sync temporary JSON file: {error}"))?;

    drop(file);

    fs::rename(&tmp_path, path)
        .map_err(|error| format!("Failed to atomically move JSON payload into place: {error}"))?;

    fsync_parent(path)
        .map_err(|error| format!("Failed to sync JSON payload parent directory: {error}"))?;

    Ok(())
}

pub(super) fn durable_write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    write_json_file_atomically(path, value)?;
    fsync_path(path).map_err(|error| format!("Failed to sync durable JSON file: {error}"))?;
    fsync_parent(path)
        .map_err(|error| format!("Failed to sync durable JSON parent directory: {error}"))?;
    Ok(())
}

pub(super) fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let file = File::open(path).map_err(|error| format!("Failed to open JSON file: {error}"))?;
    serde_json::from_reader(file).map_err(|error| format!("Failed to parse JSON file: {error}"))
}

pub(super) fn read_json_from_zip<T: for<'de> Deserialize<'de>, R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
) -> Result<T, String> {
    let file = archive
        .by_index(index)
        .map_err(|error| format!("Failed to open ZIP JSON entry: {error}"))?;
    serde_json::from_reader(file).map_err(|error| format!("Failed to parse ZIP JSON entry: {error}"))
}

pub(super) fn read_text_from_zip<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
) -> Result<String, String> {
    let mut file = archive
        .by_index(index)
        .map_err(|error| format!("Failed to open ZIP text entry: {error}"))?;

    let mut output = String::new();
    file.read_to_string(&mut output)
        .map_err(|error| format!("Failed to read ZIP text entry: {error}"))?;
    Ok(output)
}

pub(super) fn read_history_jsonl_line_bounded<R: BufRead>(
    reader: &mut R,
) -> Result<Option<String>, String> {
    // Allow one extra byte for '\n' and one extra for optional '\r' so
    // CRLF-terminated lines with exactly MAX_HISTORY_JSONL_LINE_BYTES payload
    // are still accepted.
    let max_read_bytes = MAX_HISTORY_JSONL_LINE_BYTES.saturating_add(2) as u64;
    let mut bytes = Vec::new();
    let read = {
        let mut limited_reader = reader.take(max_read_bytes);
        limited_reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| format!("Failed to read history payload line: {error}"))?
    };

    if read == 0 {
        return Ok(None);
    }

    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }

    if bytes.len() > MAX_HISTORY_JSONL_LINE_BYTES {
        return Err(format!(
            "history_jsonl_line_too_large:{}>{}",
            bytes.len(),
            MAX_HISTORY_JSONL_LINE_BYTES
        ));
    }

    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| format!("History payload line is not valid UTF-8: {error}"))
}

pub(super) fn validate_user_store_payload_shape(value: &serde_json::Value) -> Result<(), String> {
    if value.is_object() {
        Ok(())
    } else {
        Err("root JSON value must be an object".to_string())
    }
}

pub(super) fn normalize_user_stats_payload(payload: &UserStatsPayloadV1) -> UserStatsPayloadV1 {
    let mut normalized = payload.clone();
    normalized.version = USER_STATS_PAYLOAD_VERSION;
    normalized.total_words = normalized.total_words.max(0);
    normalized.total_duration_ms = normalized.total_duration_ms.max(0);
    normalized.total_transcriptions = normalized.total_transcriptions.max(0);
    normalized.current_streak_days = normalized.current_streak_days.max(0);
    normalized.total_filler_words_removed = normalized.total_filler_words_removed.max(0);
    normalized.total_speech_duration_ms = normalized
        .total_speech_duration_ms
        .max(0)
        .min(normalized.total_duration_ms);
    normalized.duration_stats_semantics_version = normalized.duration_stats_semantics_version.max(0);
    normalized
}

pub(super) fn validate_user_stats_payload(payload: &UserStatsPayloadV1) -> Result<(), String> {
    if payload.version != USER_STATS_PAYLOAD_VERSION {
        return Err(format!(
            "unsupported version {} (expected {})",
            payload.version, USER_STATS_PAYLOAD_VERSION
        ));
    }

    if payload.total_words < 0
        || payload.total_duration_ms < 0
        || payload.total_transcriptions < 0
        || payload.current_streak_days < 0
        || payload.total_filler_words_removed < 0
        || payload.total_speech_duration_ms < 0
        || payload.duration_stats_semantics_version < 0
    {
        return Err("numeric fields must be non-negative".to_string());
    }

    if payload.total_speech_duration_ms > payload.total_duration_ms {
        return Err("total_speech_duration_ms cannot exceed total_duration_ms".to_string());
    }

    for date in &payload.transcription_dates {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|error| format!("invalid transcription_dates entry '{date}': {error}"))?;
    }

    if let Some(date) = payload.current_streak_counted_through_date.as_deref() {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|error| {
            format!("invalid current_streak_counted_through_date '{date}': {error}")
        })?;
    }

    if payload.current_streak_days > 0 && payload.current_streak_counted_through_date.is_none() {
        return Err(
            "current_streak_counted_through_date is required when current_streak_days is positive"
                .to_string(),
        );
    }

    if let (Some(first), Some(last)) = (
        payload.first_transcription_date,
        payload.last_transcription_date,
    ) {
        if first > last {
            return Err("first_transcription_date cannot be greater than last_transcription_date".to_string());
        }
    }

    Ok(())
}

pub(super) fn collect_payload_files(workspace: &Path) -> Result<Vec<String>, String> {
    let mut output = Vec::new();
    collect_payload_files_recursive(workspace, workspace, &mut output)?;
    Ok(output)
}

pub(super) fn collect_payload_files_recursive(
    root: &Path,
    current: &Path,
    output: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(current)
        .map_err(|error| format!("Failed to scan payload directory '{}': {error}", current.display()))?
    {
        let entry = entry.map_err(|error| format!("Failed to read payload directory entry: {error}"))?;
        let path = entry.path();

        if path.is_dir() {
            collect_payload_files_recursive(root, &path, output)?;
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("Failed to compute payload relative path: {error}"))?
            .to_string_lossy()
            .replace('\\', "/");

        if relative == CHECKSUM_FILE {
            continue;
        }

        output.push(relative);
    }

    Ok(())
}

pub(super) fn checksum_zip_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    index: usize,
) -> Result<String, String> {
    let mut file = archive
        .by_index(index)
        .map_err(|error| format!("Failed to open ZIP entry for checksum: {error}"))?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Failed to read ZIP entry for checksum: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn checksum_path(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("Failed to open file for checksum '{}': {error}", path.display()))?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Failed to read file for checksum '{}': {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(not(test))]
fn export_payload_size_limit_bytes() -> u64 {
    MAX_PAYLOAD_FILE_SIZE_BYTES
}

#[cfg(test)]
fn export_payload_size_limit_bytes() -> u64 {
    std::env::var("HANDY_TEST_BR_MAX_PAYLOAD_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(MAX_PAYLOAD_FILE_SIZE_BYTES)
}

pub(super) fn validate_export_payload_size_from_len(
    archive_relative_path: &str,
    size_bytes: u64,
) -> Result<(), String> {
    let limit = export_payload_size_limit_bytes();
    if size_bytes > limit {
        return Err(format!(
            "export_payload_size_limit:{archive_relative_path}:{size_bytes}>{limit}"
        ));
    }

    Ok(())
}

pub(super) fn validate_export_payload_file_size(
    path: &Path,
    archive_relative_path: &str,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "Failed to inspect payload file metadata '{}' for size validation: {error}",
            path.display()
        )
    })?;

    if metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to validate symbolic link payload '{}'",
            path.display()
        ));
    }

    if !metadata.is_file() {
        return Err(format!(
            "Refusing to validate non-file payload '{}'",
            path.display()
        ));
    }

    validate_export_payload_size_from_len(archive_relative_path, metadata.len())
}

pub(super) fn copy_file_chunked(source: &Path, destination: &Path) -> Result<(), String> {
    copy_file_chunked_with_cancel(source, destination, || Ok(()))
}

pub(super) fn copy_file_chunked_with_cancel<F>(
    source: &Path,
    destination: &Path,
    mut cancel_check: F,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    let source_metadata = fs::symlink_metadata(source).map_err(|error| {
        format!(
            "Failed to inspect source file metadata '{}': {error}",
            source.display()
        )
    })?;

    if source_metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to copy symbolic link source '{}'",
            source.display()
        ));
    }

    if !source_metadata.is_file() {
        return Err(format!(
            "Refusing to copy non-file source '{}'",
            source.display()
        ));
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create destination parent directory '{}': {error}",
                parent.display()
            )
        })?;
    }

    if destination.exists() {
        let destination_metadata = fs::symlink_metadata(destination).map_err(|error| {
            format!(
                "Failed to inspect destination metadata '{}': {error}",
                destination.display()
            )
        })?;

        if destination_metadata.file_type().is_symlink() {
            return Err(format!(
                "Refusing to overwrite symbolic link destination '{}'",
                destination.display()
            ));
        }

        if !destination_metadata.is_file() {
            return Err(format!(
                "Refusing to overwrite non-file destination '{}'",
                destination.display()
            ));
        }
    }

    let mut input = File::open(source)
        .map_err(|error| format!("Failed to open source file '{}': {error}", source.display()))?;
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(destination)
        .map_err(|error| {
            format!(
                "Failed to open destination file '{}': {error}",
                destination.display()
            )
        })?;

    let mut buffer = vec![0_u8; 1024 * 1024];
    cancel_check()?;
    loop {
        cancel_check()?;
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("Failed to read source file '{}': {error}", source.display()))?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read]).map_err(|error| {
            format!(
                "Failed to write destination file '{}': {error}",
                destination.display()
            )
        })?;
    }

    output
        .flush()
        .map_err(|error| format!("Failed to flush destination file '{}': {error}", destination.display()))?;
    output
        .sync_all()
        .map_err(|error| format!("Failed to sync destination file '{}': {error}", destination.display()))?;

    Ok(())
}

pub(super) fn copy_dir_recursive_chunked(source: &Path, destination: &Path) -> Result<(), String> {
    let mut no_cancel = || Ok(());
    copy_dir_recursive_chunked_with_cancel(source, destination, &mut no_cancel)
}

pub(super) fn copy_dir_recursive_chunked_with_cancel<F>(
    source: &Path,
    destination: &Path,
    cancel_check: &mut F,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    cancel_check()?;

    let source_metadata = fs::symlink_metadata(source).map_err(|error| {
        format!(
            "Failed to inspect source directory metadata '{}': {error}",
            source.display()
        )
    })?;
    if source_metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to copy symbolic link source directory '{}'",
            source.display()
        ));
    }
    if !source_metadata.is_dir() {
        return Err(format!(
            "Refusing to copy non-directory source '{}'",
            source.display()
        ));
    }

    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "Failed to create destination directory '{}': {error}",
            destination.display()
        )
    })?;

    for entry in fs::read_dir(source)
        .map_err(|error| format!("Failed to read source directory '{}': {error}", source.display()))?
    {
        cancel_check()?;
        let entry = entry.map_err(|error| format!("Failed to read source directory entry: {error}"))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect source entry type: {error}"))?;

        if file_type.is_symlink() {
            return Err(format!(
                "Refusing to copy symbolic link entry '{}'",
                source_path.display()
            ));
        }

        if file_type.is_dir() {
            copy_dir_recursive_chunked_with_cancel(&source_path, &destination_path, cancel_check)?;
        } else if file_type.is_file() {
            copy_file_chunked_with_cancel(&source_path, &destination_path, &mut *cancel_check)?;
        } else {
            return Err(format!(
                "Refusing to copy unsupported entry type '{}'",
                source_path.display()
            ));
        }
    }

    Ok(())
}

pub(super) fn remove_file_with_parent_sync(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("Failed to remove file '{}': {error}", path.display()))?;
    }

    fsync_parent(path)
        .map_err(|error| format!("Failed to sync parent after removing '{}': {error}", path.display()))?;

    Ok(())
}

pub(super) fn normalize_archive_path(raw: &str) -> Result<String, String> {
    let normalized_separators = raw.replace('\\', "/");

    let starts_with_windows_drive = normalized_separators
        .chars()
        .nth(1)
        .map(|c| c == ':')
        .unwrap_or(false)
        && normalized_separators
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false);
    if starts_with_windows_drive {
        return Err("windows drive-prefixed paths are not allowed".to_string());
    }

    if normalized_separators.starts_with('/') {
        return Err("absolute paths are not allowed".to_string());
    }

    let mut parts = Vec::new();
    for segment in normalized_separators.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err("parent traversal is not allowed".to_string());
        }
        if segment.chars().any(|c| c.is_control()) {
            return Err("control characters are not allowed in path segments".to_string());
        }
        parts.push(segment);
    }

    if parts.is_empty() {
        return Err("empty normalized path is not allowed".to_string());
    }

    Ok(parts.join("/"))
}

pub(super) fn sanitize_relative_file_name(file_name: &str) -> Result<String, String> {
    let normalized = normalize_archive_path(file_name)
        .map_err(|error| format!("Unsafe recording file name '{file_name}': {error}"))?;
    if normalized.contains('/') {
        return Err(format!(
            "Recording file name '{file_name}' resolved to nested path '{normalized}', which is not allowed"
        ));
    }
    Ok(normalized)
}

pub(super) fn fsync_parent(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fsync_path(parent)
    } else {
        Ok(())
    }
}

#[cfg(windows)]
pub(super) fn fsync_path(path: &Path) -> io::Result<()> {
    // Windows requires special flags to open directory handles. Until backup/restore
    // is supported on Windows, skip directory fsync and keep file fsync behavior.
    if path.is_dir() {
        return Ok(());
    }
    let file = File::open(path)?;
    file.sync_all()
}

#[cfg(not(windows))]
pub(super) fn fsync_path(path: &Path) -> io::Result<()> {
    let file = File::open(path)?;
    file.sync_all()
}

pub(super) fn current_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}

pub(super) fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

pub(super) fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

pub(super) fn timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub(super) fn count_files(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path)
        .map_err(|error| format!("Failed to read directory '{}': {error}", path.display()))?
    {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect file type: {error}"))?;
        if file_type.is_dir() {
            total = total.saturating_add(count_files(&entry.path())?);
        } else {
            total = total.saturating_add(1);
        }
    }

    Ok(total)
}

pub(super) fn managed_data_size_bytes(app_data_dir: &Path) -> Result<u64, String> {
    let mut total = 0_u64;

    for file_name in [HISTORY_DB_FILE, USER_DICTIONARY_FILE, USER_STORE_DB_FILE] {
        let path = app_data_dir.join(file_name);
        if !path.exists() {
            continue;
        }
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("Failed to stat managed file '{}': {error}", path.display()))?;
        if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        }
    }

    total = total.saturating_add(directory_size_bytes(&app_data_dir.join(RECORDINGS_DIR))?);
    Ok(total)
}

pub(super) fn directory_size_bytes(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path)
        .map_err(|error| format!("Failed to read directory '{}': {error}", path.display()))?
    {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect file type: {error}"))?;
        if file_type.is_dir() {
            total = total.saturating_add(directory_size_bytes(&entry_path)?);
        } else {
            let metadata = entry
                .metadata()
                .map_err(|error| format!("Failed to stat '{}': {error}", entry_path.display()))?;
            total = total.saturating_add(metadata.len());
        }
    }

    Ok(total)
}
