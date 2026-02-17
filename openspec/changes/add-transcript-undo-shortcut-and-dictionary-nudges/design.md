## Context
Handy needs a strict undo path for the most recent app-originated paste so users can quickly recover from bad transcription output.

The prior design included proactive dictionary nudges and a frontend heuristic bridge. That scope is intentionally removed. History sparkle suggestions remain the only dictionary suggestion surface.

## Goals / Non-Goals
- Goals:
  - Strict undo for last tracked paste.
  - Clear undo/no-op feedback.
  - Preserve cancel-first behavior during active operations.
  - Preserve discoverability hint and stats rollback semantics.
- Non-Goals:
  - Undo-driven alias or unresolved nudges.
  - Rust<->frontend heuristic evaluation bridge.
  - Dictionary prefill intent routing from undo.

## Key Decisions
- Decision: undo scope is feedback-only.
  - Reason: reliability and maintainability.
- Decision: keep existing History alias suggestion heuristics untouched.
  - Reason: users already have contextual correction flow there.
- Decision: persist discoverability evidence in `settings_store.json` under a dedicated key (`undo_discoverability`).
  - Persisted fields:
    - `has_seen_undo_hint`
    - `successful_paste_count`
    - `has_used_undo`

## Data Model
### RecentPasteSlot (in-memory)
- `paste_id`
- `source_action`
- `stats_token`
- `auto_refined`
- `pasted_text`
- `suggestion_text`
- `created_at_ms`
- `expires_at_ms`
- `consumed`

Rules:
- single-slot overwrite
- TTL = 120s
- single-use consume on dispatch attempt

### UndoDiscoverabilityEvidence (persisted)
- storage: `settings_store.json` key `undo_discoverability`
- fields:
  - `has_seen_undo_hint: bool`
  - `successful_paste_count: u32`
  - `has_used_undo: bool`

Unknown legacy fields in existing payload are ignored during deserialize.

## Behavioral Flow
### Undo action
1. If active recording/transcription/refine/post-process (or stop-transition grace) is present, trigger cancellation path and short-circuit.
2. Otherwise evaluate slot validity.
3. Valid slot:
  - wait modifier-release delay
  - dispatch one platform undo chord
  - consume slot
  - request stats rollback for transcribe-origin slots
  - mark `has_used_undo=true`
  - emit `Undo applied` feedback
4. Missing/consumed slot emits `Nothing to undo`.
5. Expired slot emits `Undo expired` once and clears slot.

### Discoverability
- after second successful tracked paste, schedule one-time hint (2.5s delay) only when:
  - `has_seen_undo_hint=false`
  - `has_used_undo=false`

## Logging Scope
Keep logs for:
- slot lifecycle
- dispatch/no-op reasons
- operation cancellation
- stats rollback lifecycle
- discoverability scheduling/skips/emission

Remove all bridge/nudge/cluster/prefill-specific log requirements.

## Risks / Trade-offs
- Removed proactive nudges reduce automated correction prompts.
- Mitigation: retain History sparkle suggestion flow as primary dictionary correction mechanism.
