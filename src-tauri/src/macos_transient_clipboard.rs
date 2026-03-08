use std::ffi::CString;
use std::os::raw::c_char;
use tracing::debug;

const DEFAULT_TRANSIENT_PASTEBOARD_TIMEOUT_MS: u64 = 3_000;

extern "C" {
    fn stage_transient_pasteboard_text(
        text: *const c_char,
        backup_text: *const c_char,
        timeout_ms: u64,
    ) -> i32;
}

pub fn stage_transient_text(text: &str, backup_text: Option<&str>) -> Result<(), String> {
    stage_transient_text_with_timeout(text, backup_text, DEFAULT_TRANSIENT_PASTEBOARD_TIMEOUT_MS)
}

pub fn stage_transient_text_with_timeout(
    text: &str,
    backup_text: Option<&str>,
    timeout_ms: u64,
) -> Result<(), String> {
    let text_cstr = CString::new(text).map_err(|err| format!("Invalid transcript text: {err}"))?;
    let backup_cstr = backup_text
        .map(|value| CString::new(value).map_err(|err| format!("Invalid clipboard backup: {err}")))
        .transpose()?;

    debug!(
        text_chars = text.chars().count(),
        backup_present = backup_text.is_some(),
        timeout_ms,
        "Calling transient pasteboard FFI"
    );

    let status = unsafe {
        stage_transient_pasteboard_text(
            text_cstr.as_ptr(),
            backup_cstr
                .as_ref()
                .map_or(std::ptr::null(), |value| value.as_ptr()),
            timeout_ms,
        )
    };

    debug!(status, "Transient pasteboard FFI returned");

    map_stage_status(status)
}

fn map_stage_status(status: i32) -> Result<(), String> {
    match status {
        0 => Ok(()),
        -1 => Err("Transient pasteboard staging failed: missing text pointer".to_string()),
        -2 => Err("Transient pasteboard staging failed: text is empty".to_string()),
        -3 => Err("Transient pasteboard staging failed: provider registration was rejected".to_string()),
        -4 => Err("Transient pasteboard staging failed: AppKit refused to write the pasteboard item".to_string()),
        other => Err(format!(
            "Transient pasteboard staging failed with unexpected status {other}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::map_stage_status;

    #[test]
    fn stage_status_zero_is_success() {
        assert!(map_stage_status(0).is_ok());
    }

    #[test]
    fn stage_status_known_errors_map_to_messages() {
        assert_eq!(
            map_stage_status(-1).unwrap_err(),
            "Transient pasteboard staging failed: missing text pointer"
        );
        assert_eq!(
            map_stage_status(-2).unwrap_err(),
            "Transient pasteboard staging failed: text is empty"
        );
        assert_eq!(
            map_stage_status(-3).unwrap_err(),
            "Transient pasteboard staging failed: provider registration was rejected"
        );
        assert_eq!(
            map_stage_status(-4).unwrap_err(),
            "Transient pasteboard staging failed: AppKit refused to write the pasteboard item"
        );
    }

    #[test]
    fn stage_status_unknown_error_includes_code() {
        assert_eq!(
            map_stage_status(-9).unwrap_err(),
            "Transient pasteboard staging failed with unexpected status -9"
        );
    }
}
