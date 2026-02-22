use once_cell::sync::OnceCell;
use sentry::protocol::{Context, Map as SentryMap, User};
use sentry::types::Uuid as SentryUuid;
use serde_json::json;
use std::error::Error;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tracing::{debug, warn};
use uuid::Uuid;

const OBSERVABILITY_STORE_PATH: &str = "observability_store.json";
const ANONYMOUS_INSTALL_ID_KEY: &str = "anonymous_install_id";
const ANONYMOUS_INSTALL_CREATED_AT_KEY: &str = "anonymous_install_created_at";
const RUN_ID_EXTRA_KEY: &str = "run_id";
const INSTALL_ID_VERSION_EXTRA_KEY: &str = "install_id_version";
const DIAGNOSTICS_CONTEXT_KEY: &str = "diagnostics";
const INSTALL_ID_VERSION: u8 = 1;

static APP_RUN_ID: OnceCell<String> = OnceCell::new();

#[derive(Clone, Copy)]
pub struct HandledErrorMeta {
    pub component: &'static str,
    pub operation: &'static str,
    pub level: sentry::Level,
}

pub fn get_or_create_run_id() -> &'static str {
    APP_RUN_ID
        .get_or_init(|| Uuid::new_v4().to_string())
        .as_str()
}

pub fn get_or_create_anonymous_install_id(app: &AppHandle) -> Result<String, String> {
    let store = app
        .store(OBSERVABILITY_STORE_PATH)
        .map_err(|error| format!("Failed to initialize observability store: {error}"))?;

    if let Some(existing) = store
        .get(ANONYMOUS_INSTALL_ID_KEY)
        .and_then(|value| value.as_str().map(ToString::to_string))
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(existing);
    }

    let install_id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    store.set(ANONYMOUS_INSTALL_ID_KEY, json!(install_id));
    store.set(ANONYMOUS_INSTALL_CREATED_AT_KEY, json!(created_at));

    // The store auto-saves, but force-save on first creation so this ID survives
    // abrupt app termination during initial setup.
    if let Err(error) = store.save() {
        warn!("Failed to persist anonymous install ID immediately: {}", error);
    }

    Ok(install_id)
}

pub fn initialize_sentry_identity_scope(app: &AppHandle) {
    let sentry_active = sentry::Hub::with_active(|hub| hub.client().is_some());
    if !sentry_active {
        debug!(
            "Skipping Sentry identity scope setup because no active Sentry client is bound"
        );
        return;
    }

    let install_id = match get_or_create_anonymous_install_id(app) {
        Ok(value) => value,
        Err(error) => {
            warn!(
                "Skipping anonymous install correlation setup due to store error: {}",
                error
            );
            return;
        }
    };

    let anonymous_user_id = format!("anon:{install_id}");
    let run_id = get_or_create_run_id();

    sentry::configure_scope(|scope| {
        scope.set_user(Some(User {
            id: Some(anonymous_user_id.clone()),
            ..Default::default()
        }));
        scope.set_extra(RUN_ID_EXTRA_KEY, json!(run_id));
        scope.set_extra(INSTALL_ID_VERSION_EXTRA_KEY, json!(INSTALL_ID_VERSION));
        scope.set_context(DIAGNOSTICS_CONTEXT_KEY, diagnostics_context(run_id));
    });
}

pub fn capture_handled_error(
    meta: &HandledErrorMeta,
    err: &(dyn Error + 'static),
) -> SentryUuid {
    sentry::with_scope(
        |scope| apply_handled_error_scope(scope, meta),
        || sentry::capture_error(err),
    )
}

pub fn capture_handled_message(meta: &HandledErrorMeta, message: &str) -> SentryUuid {
    sentry::with_scope(
        |scope| apply_handled_error_scope(scope, meta),
        || sentry::capture_message(message, meta.level),
    )
}

fn apply_handled_error_scope(scope: &mut sentry::Scope, meta: &HandledErrorMeta) {
    scope.set_level(Some(meta.level));
    scope.set_tag("handled", "true");
    scope.set_tag("component", meta.component);
    scope.set_tag("operation", meta.operation);
    scope.set_fingerprint(Some(&["{{ default }}", meta.component, meta.operation]));

    let run_id = get_or_create_run_id();
    scope.set_extra(RUN_ID_EXTRA_KEY, json!(run_id));
}

fn diagnostics_context(run_id: &str) -> Context {
    let mut context = SentryMap::new();
    context.insert(
        RUN_ID_EXTRA_KEY.to_string(),
        json!(run_id),
    );
    context.insert(
        INSTALL_ID_VERSION_EXTRA_KEY.to_string(),
        json!(INSTALL_ID_VERSION),
    );
    Context::Other(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_is_stable_during_process_lifetime() {
        let first = get_or_create_run_id().to_string();
        let second = get_or_create_run_id().to_string();
        assert_eq!(first, second);
    }

    #[test]
    fn diagnostics_context_contains_expected_keys() {
        let context = diagnostics_context("run-abc");
        match context {
            Context::Other(values) => {
                assert_eq!(values.get("run_id"), Some(&json!("run-abc")));
                assert_eq!(values.get("install_id_version"), Some(&json!(1)));
            }
            _ => panic!("expected diagnostics context to use Context::Other"),
        }
    }
}
