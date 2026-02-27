use crate::dictionary_normalization::normalized_dictionary_len;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Manager};
use tracing::warn;

pub const USER_DICTIONARY_FILE_NAME: &str = "user_dictionary.json";
const USER_DICTIONARY_VERSION: u32 = 1;
const SINGLE_WORD_FUZZY_BLOCK_MAX_NORMALIZED_LEN: usize = 4;

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct CustomWordEntry {
    pub input: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub replacement: String,
    pub is_replacement: bool,
    #[serde(default)]
    pub fuzzy_enabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DictionaryEnvelope {
    #[serde(default = "default_dictionary_version")]
    version: u32,
    #[serde(default)]
    entries: Vec<CustomWordEntry>,
}

fn default_dictionary_version() -> u32 {
    USER_DICTIONARY_VERSION
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionaryLoadStatus {
    Loaded,
    MissingFile,
    Malformed,
    UnsupportedVersion(u32),
    IoError,
}

pub struct DictionaryState {
    write_gate: Mutex<()>,
    entries: RwLock<Arc<Vec<CustomWordEntry>>>,
    last_load_status: RwLock<DictionaryLoadStatus>,
}

impl DictionaryState {
    fn new(entries: Vec<CustomWordEntry>, status: DictionaryLoadStatus) -> Self {
        Self {
            write_gate: Mutex::new(()),
            entries: RwLock::new(Arc::new(entries)),
            last_load_status: RwLock::new(status),
        }
    }

    pub fn snapshot(&self) -> Arc<Vec<CustomWordEntry>> {
        self.entries
            .read()
            .map(|entries| entries.clone())
            .unwrap_or_else(|_| Arc::new(Vec::new()))
    }

    fn replace_entries(&self, next_entries: Vec<CustomWordEntry>) -> Result<(), String> {
        let mut entries_guard = self
            .entries
            .write()
            .map_err(|_| "Failed to acquire dictionary write lock".to_string())?;
        *entries_guard = Arc::new(next_entries);
        Ok(())
    }

    fn set_last_load_status(&self, status: DictionaryLoadStatus) -> Result<(), String> {
        let mut status_guard = self
            .last_load_status
            .write()
            .map_err(|_| "Failed to acquire dictionary status lock".to_string())?;
        *status_guard = status;
        Ok(())
    }
}

fn migrate_dictionary_entry(entry: &mut CustomWordEntry) -> bool {
    let mut changed = false;
    let normalized_len = normalized_dictionary_len(&entry.input);
    let canonical_word_count = entry.input.split_whitespace().count();
    let is_short_single_word_target = canonical_word_count == 1
        && normalized_len <= SINGLE_WORD_FUZZY_BLOCK_MAX_NORMALIZED_LEN;

    if entry.is_replacement {
        if entry.fuzzy_enabled != Some(false) {
            entry.fuzzy_enabled = Some(false);
            changed = true;
        }
        return changed;
    }

    if is_short_single_word_target {
        if entry.fuzzy_enabled != Some(false) {
            entry.fuzzy_enabled = Some(false);
            changed = true;
        }
        return changed;
    }

    if entry.fuzzy_enabled.is_none() {
        entry.fuzzy_enabled = Some(true);
        changed = true;
    }

    changed
}

fn migrate_dictionary_entries(entries: &mut [CustomWordEntry]) {
    for entry in entries {
        let _ = migrate_dictionary_entry(entry);
    }
}

fn dictionary_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir for dictionary: {e}"))?;
    Ok(app_data_dir.join(USER_DICTIONARY_FILE_NAME))
}

fn parent_dir_fsync(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        let parent_file = File::open(parent)?;
        parent_file.sync_all()?;
    }
    Ok(())
}

fn write_entries_to_path_with_dir_sync<F>(
    path: &Path,
    entries: &[CustomWordEntry],
    dir_sync: F,
) -> Result<(), String>
where
    F: Fn(&Path) -> std::io::Result<()>,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create dictionary directory: {e}"))?;
    }

    let envelope = DictionaryEnvelope {
        version: USER_DICTIONARY_VERSION,
        entries: entries.to_vec(),
    };
    let encoded = serde_json::to_vec_pretty(&envelope)
        .map_err(|e| format!("Failed to serialize dictionary entries: {e}"))?;

    let temp_name = format!(
        ".{}.tmp-{}-{}",
        USER_DICTIONARY_FILE_NAME,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    let temp_path = path.with_file_name(temp_name);

    let mut temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|e| format!("Failed to create temporary dictionary file: {e}"))?;

    temp_file
        .write_all(&encoded)
        .map_err(|e| format!("Failed to write dictionary temp file: {e}"))?;
    temp_file
        .flush()
        .map_err(|e| format!("Failed to flush dictionary temp file: {e}"))?;
    temp_file
        .sync_all()
        .map_err(|e| format!("Failed to sync dictionary temp file: {e}"))?;
    drop(temp_file);

    fs::rename(&temp_path, path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        format!("Failed to atomically rename dictionary file: {e}")
    })?;

    if let Err(err) = dir_sync(path) {
        warn!(
            event_code = "dictionary_parent_sync_failed",
            error = %err,
            "Dictionary file renamed successfully but parent directory fsync failed"
        );
    }

    Ok(())
}

fn write_entries_to_path(path: &Path, entries: &[CustomWordEntry]) -> Result<(), String> {
    write_entries_to_path_with_dir_sync(path, entries, parent_dir_fsync)
}

fn load_entries_from_path(path: &Path) -> (Vec<CustomWordEntry>, DictionaryLoadStatus) {
    if !path.exists() {
        return (Vec::new(), DictionaryLoadStatus::MissingFile);
    }

    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => {
            warn!(
                event_code = "dictionary_read_failed",
                error = %err,
                path = %path.display(),
                "Failed to read dictionary file; falling back to empty dictionary"
            );
            return (Vec::new(), DictionaryLoadStatus::IoError);
        }
    };

    let envelope: DictionaryEnvelope = match serde_json::from_str(&raw) {
        Ok(envelope) => envelope,
        Err(err) => {
            warn!(
                event_code = "dictionary_parse_failed",
                error = %err,
                path = %path.display(),
                "Failed to parse dictionary file; falling back to empty dictionary"
            );
            return (Vec::new(), DictionaryLoadStatus::Malformed);
        }
    };

    if envelope.version != USER_DICTIONARY_VERSION {
        warn!(
            event_code = "dictionary_unsupported_version",
            found_version = envelope.version,
            expected_version = USER_DICTIONARY_VERSION,
            path = %path.display(),
            "Unsupported dictionary version; falling back to empty dictionary"
        );
        return (
            Vec::new(),
            DictionaryLoadStatus::UnsupportedVersion(envelope.version),
        );
    }

    let mut entries = envelope.entries;
    migrate_dictionary_entries(&mut entries);
    (entries, DictionaryLoadStatus::Loaded)
}

pub fn initialize_dictionary_state(app: &AppHandle) -> Arc<DictionaryState> {
    let path = match dictionary_path(app) {
        Ok(path) => path,
        Err(err) => {
            warn!(
                event_code = "dictionary_path_resolve_failed",
                error = %err,
                "Failed to resolve dictionary path; using empty dictionary state"
            );
            return Arc::new(DictionaryState::new(Vec::new(), DictionaryLoadStatus::IoError));
        }
    };

    let (entries, status) = load_entries_from_path(&path);
    Arc::new(DictionaryState::new(entries, status))
}

pub fn get_dictionary_snapshot(app: &AppHandle) -> Arc<Vec<CustomWordEntry>> {
    if let Some(state) = app.try_state::<Arc<DictionaryState>>() {
        return state.snapshot();
    }

    warn!(
        event_code = "dictionary_state_missing",
        "Dictionary state is not initialized; returning empty snapshot"
    );
    Arc::new(Vec::new())
}

pub fn set_dictionary_entries(app: &AppHandle, entries: Vec<CustomWordEntry>) -> Result<(), String> {
    let Some(state) = app.try_state::<Arc<DictionaryState>>() else {
        return Err("Dictionary state is not initialized".to_string());
    };

    let _write_guard = state
        .write_gate
        .lock()
        .map_err(|_| "Failed to acquire dictionary write gate".to_string())?;

    let path = dictionary_path(app)?;
    persist_then_swap_entries(state.inner(), &path, entries, write_entries_to_path)
}

fn persist_then_swap_entries<W>(
    state: &DictionaryState,
    path: &Path,
    entries: Vec<CustomWordEntry>,
    writer: W,
) -> Result<(), String>
where
    W: Fn(&Path, &[CustomWordEntry]) -> Result<(), String>,
{
    writer(path, &entries)?;
    state.replace_entries(entries)?;
    state.set_last_load_status(DictionaryLoadStatus::Loaded)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    fn dictionary_entry(
        input: &str,
        is_replacement: bool,
        fuzzy_enabled: Option<bool>,
    ) -> CustomWordEntry {
        CustomWordEntry {
            input: input.to_string(),
            aliases: Vec::new(),
            replacement: input.to_string(),
            is_replacement,
            fuzzy_enabled,
        }
    }

    #[test]
    fn load_entries_from_missing_file_returns_empty() {
        let temp = TempDir::new().expect("create temp dir");
        let path = temp.path().join("missing-user-dictionary.json");
        let (entries, status) = load_entries_from_path(&path);
        assert!(entries.is_empty());
        assert_eq!(status, DictionaryLoadStatus::MissingFile);
    }

    #[test]
    fn load_entries_with_unsupported_version_returns_empty() {
        let temp = TempDir::new().expect("create temp dir");
        let path = temp.path().join("user_dictionary.json");
        fs::write(&path, r#"{ "version": 99, "entries": [] }"#).expect("write unsupported file");

        let (entries, status) = load_entries_from_path(&path);
        assert!(entries.is_empty());
        assert_eq!(status, DictionaryLoadStatus::UnsupportedVersion(99));
    }

    #[test]
    fn load_entries_with_malformed_json_returns_empty() {
        let temp = TempDir::new().expect("create temp dir");
        let path = temp.path().join("user_dictionary.json");
        fs::write(&path, "{ malformed json").expect("write malformed file");

        let (entries, status) = load_entries_from_path(&path);
        assert!(entries.is_empty());
        assert_eq!(status, DictionaryLoadStatus::Malformed);
    }

    #[test]
    fn load_entries_without_entries_field_defaults_to_empty() {
        let temp = TempDir::new().expect("create temp dir");
        let path = temp.path().join("user_dictionary.json");
        fs::write(&path, r#"{ "version": 1 }"#).expect("write dictionary file");

        let (entries, status) = load_entries_from_path(&path);
        assert!(entries.is_empty());
        assert_eq!(status, DictionaryLoadStatus::Loaded);
    }

    #[test]
    fn persist_keeps_success_when_parent_fsync_fails() {
        let temp = TempDir::new().expect("create temp dir");
        let path = temp.path().join("user_dictionary.json");
        let entries = vec![dictionary_entry("chat gpt", true, Some(false))];

        let result = write_entries_to_path_with_dir_sync(&path, &entries, |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "simulated dir sync failure",
            ))
        });
        assert!(result.is_ok());

        let (loaded_entries, status) = load_entries_from_path(&path);
        assert_eq!(status, DictionaryLoadStatus::Loaded);
        assert_eq!(loaded_entries.len(), 1);
    }

    #[test]
    fn migrate_entry_short_legacy_becomes_exact() {
        let mut entry = dictionary_entry("qwen", false, None);
        let changed = migrate_dictionary_entry(&mut entry);
        assert!(changed);
        assert_eq!(entry.fuzzy_enabled, Some(false));
    }

    #[test]
    fn persist_failure_keeps_in_memory_entries_unchanged() {
        let temp = TempDir::new().expect("create temp dir");
        let path = temp.path().join("user_dictionary.json");
        let state = DictionaryState::new(
            vec![dictionary_entry("legacy", true, Some(false))],
            DictionaryLoadStatus::Loaded,
        );
        let next_entries = vec![dictionary_entry("new", true, Some(false))];

        let result = persist_then_swap_entries(&state, &path, next_entries, |_path, _entries| {
            Err("simulated write failure".to_string())
        });
        assert!(result.is_err());
        assert_eq!(state.snapshot()[0].input, "legacy");
    }

    #[test]
    fn persist_success_writes_then_swaps_in_memory_entries() {
        let temp = TempDir::new().expect("create temp dir");
        let path = temp.path().join("user_dictionary.json");
        let state = DictionaryState::new(
            vec![dictionary_entry("legacy", true, Some(false))],
            DictionaryLoadStatus::Loaded,
        );
        let next_entries = vec![dictionary_entry("new", true, Some(false))];

        let result = persist_then_swap_entries(&state, &path, next_entries, |path, entries| {
            assert_eq!(state.snapshot()[0].input, "legacy");
            write_entries_to_path(path, entries)
        });
        assert!(result.is_ok());
        assert_eq!(state.snapshot()[0].input, "new");

        let (loaded_entries, status) = load_entries_from_path(&path);
        assert_eq!(status, DictionaryLoadStatus::Loaded);
        assert_eq!(loaded_entries[0].input, "new");
    }

    #[test]
    fn write_gate_serializes_writers() {
        let state = Arc::new(DictionaryState::new(Vec::new(), DictionaryLoadStatus::Loaded));
        let second_thread_started = Arc::new(AtomicBool::new(false));
        let second_thread_acquired = Arc::new(AtomicBool::new(false));

        let first_guard = state.write_gate.lock().expect("acquire first write gate");

        let state_for_thread = Arc::clone(&state);
        let started_for_thread = Arc::clone(&second_thread_started);
        let acquired_for_thread = Arc::clone(&second_thread_acquired);
        let handle = thread::spawn(move || {
            started_for_thread.store(true, Ordering::SeqCst);
            let _second_guard = state_for_thread
                .write_gate
                .lock()
                .expect("acquire second write gate");
            acquired_for_thread.store(true, Ordering::SeqCst);
        });

        for _ in 0..20 {
            if second_thread_started.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(second_thread_started.load(Ordering::SeqCst));
        thread::sleep(Duration::from_millis(20));
        assert!(!second_thread_acquired.load(Ordering::SeqCst));

        drop(first_guard);
        handle.join().expect("join writer thread");
        assert!(second_thread_acquired.load(Ordering::SeqCst));
    }
}
