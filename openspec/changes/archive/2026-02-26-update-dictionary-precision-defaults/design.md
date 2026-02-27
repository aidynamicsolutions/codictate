## Context
Dictionary matching currently runs exact-first and then fuzzy/phonetic scoring for non-replacement entries. Because fuzzy is effectively active for vocabulary entries, short/common terms can be rewritten unexpectedly.

The system also needs migration-aware behavior for persisted dictionary entries. A plain `bool` cannot distinguish missing legacy fields from explicit user choice after deserialization.

## Goals / Non-Goals
- **Goals**
  - Make exact canonical/alias behavior the default trust path.
  - Require explicit opt-in for fuzzy per entry.
  - Block high-risk short-token fuzzy rewrites.
  - Preserve deterministic upgrade behavior for existing settings.
  - Keep current global thresholds unchanged.
- **Non-Goals**
  - Remove fuzzy matching entirely.
  - Change split threshold defaults in this change.
  - Introduce additional user-facing global threshold controls.

## Decisions
- **Decision 1: Tri-state fuzzy setting for migration correctness**
  - Add `fuzzy_enabled: Option<bool>` to `CustomWordEntry`.
  - Semantics:
    - `None` = legacy/unset state.
    - `Some(true)` = explicit fuzzy opt-in.
    - `Some(false)` = explicit fuzzy off.
  - Matcher contract: only `Some(true)` enables fuzzy path.

- **Decision 2: Hard short-target fuzzy safety guard**
  - For single-word fuzzy candidates, block when normalized character length `<= 4`.
  - Apply this to both canonical input and active alias target shape.
  - This guard is absolute and takes precedence over user opt-in.

- **Decision 3: Shared normalization for consistency**
  - Extract dictionary normalization into a shared module used by both settings migration and matcher guards.
  - Use one normalization shape for all cutoff checks to avoid raw-length inconsistencies.

- **Decision 4: Legacy migration policy in settings load paths**
  - Run migration in both settings load entrypoints.
  - Rules:
    - If replacement entry: force `Some(false)`.
    - If `None` and canonical is single-word with normalized character length `<= 4`: set `Some(false)`.
    - If `None` for all other non-replacement entries: set `Some(true)`.
    - If `Some(true)` and canonical is single-word with normalized character length `<= 4`: coerce to `Some(false)`.
  - Persist settings when migration changes are applied.

- **Decision 5: Identity semantics unchanged**
  - Do not include `fuzzy_enabled` in dictionary entry identity/duplicate keys.
  - Entries differing only by fuzzy toggle remain the same logical term.

## Risks / Trade-offs
- **Risk**: Some users may expect fuzzy to continue for short terms.
  - **Mitigation**: Hard guard is intentional safety policy; docs and UI explain rationale.
- **Risk**: Legacy long terms might lose fuzzy if migration is skipped in one path.
  - **Mitigation**: run migration in both load paths and test both.
- **Risk**: Added field requires generated type updates.
  - **Mitigation**: regenerate `src/bindings.ts` via tauri-specta export in debug build.

## Migration Plan
1. Load settings and deserialize with `Option<bool>` field.
2. Apply migration helper to all dictionary entries.
3. Save settings immediately if any entry is changed.
4. All newly created entries from UI set explicit `Some(false)` or `Some(true)` (serialized boolean).

## Testing Strategy
- Unit tests for matcher short-target guard precedence.
- Unit tests for exact alias/canonical behavior unaffected.
- Unit tests for migration rules (`None -> Some(...)`, coercions).
- Regression test for `went -> qwen` style prevention.
- Type/build checks to validate generated bindings include optional `fuzzy_enabled`.
