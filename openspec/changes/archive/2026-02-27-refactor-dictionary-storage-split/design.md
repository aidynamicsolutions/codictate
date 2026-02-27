## Context
Dictionary data currently lives inside settings payload (`settings_store.json -> settings.dictionary`). Runtime consumers in transcription/correction access dictionary through settings reads.

This change decouples dictionary from settings and creates an independent persistence and runtime path.

## Goals
- Dictionary is persisted independently of settings.
- Runtime dictionary reads avoid per-request file I/O.
- Dictionary write path is deterministic and crash-safe.
- Settings reset does not affect dictionary.
- No legacy migration/import complexity.

## Non-Goals
- Backfill/import legacy dictionary entries from settings payload.
- Cleanup/removal of stale legacy dictionary bytes from `settings_store.json`.
- Backward compatibility for pre-split API shape.

## Architecture

### Data Model
- New dictionary envelope in `user_dictionary.json`:
  - `version: u32`
  - `entries: Vec<CustomWordEntry>` (`entries` defaults to empty when omitted)
- Supported version: `1`.

Version behavior:
- `version == 1`: load entries.
- unknown/newer version: log warning with explicit diagnostic code and load empty entries.

### Runtime State
Use managed app state for dictionary:
- `write_gate: Mutex<()>` for serializing full write operations.
- `entries: RwLock<Arc<Vec<CustomWordEntry>>>` for short-lived snapshot reads/swaps.

Initialization:
- Load dictionary once during app startup before command handling/manager use.
- If load fails/malformed/version-unsupported, initialize empty entries and warn.

Runtime access:
- transcription/correction read dictionary from in-memory snapshot only.

### Write Consistency Contract
`set_user_dictionary` uses disk-write-first policy:
1. Acquire `write_gate`.
2. Persist to disk atomically.
3. If persist succeeds, swap in-memory entries under short lock.
4. If persist fails, return error and keep in-memory entries unchanged.

This guarantees deterministic last-write-wins order by serialized completion and avoids state divergence on write failure.

### Crash-Safe Persistence
Target path: `app_data_dir()/user_dictionary.json`.

Atomic replace requirements:
- temp file must be created in the same directory as target
- write payload then flush + `sync_all`
- rename temp -> target only after successful sync
- attempt best-effort parent directory fsync after rename

Failure semantics:
- pre-rename failure leaves previous file intact
- directory fsync failure after successful rename logs warning but operation remains successful

### Settings Boundary
- Remove `dictionary` from `AppSettings`.
- Remove settings-side dictionary migration logic.
- Do not mutate legacy settings payload for cleanup.

Operational note:
- stale legacy `settings.dictionary` bytes may remain on disk until settings payload is rewritten/reset.

### Reset Boundary
- `reset_app_settings` resets settings payload only.
- Dictionary file and dictionary runtime state remain unchanged.

## Risks and Tradeoffs
- Unknown/newer dictionary version fallback to empty may cause overwrite on subsequent successful save.
  - Accepted for pre-production hard-reset scope.
- Stale legacy dictionary bytes remain in settings blob.
  - Accepted, documented lifecycle tradeoff.

## Validation Strategy
- Strict OpenSpec validation for change package.
- Rust unit/integration tests for load/write/version/error/serialization behavior.
- Runtime verification for transcription/correction memory reads.
- Frontend checks for dictionary CRUD and history alias action after state split.

