## Context
`paste_last_transcript` currently calls the shared paste path, which always runs smart insertion preparation when enabled globally. This is ideal for live dictation and refine flows, but creates non-deterministic replay for paste-last.

## Goals / Non-Goals
- Goals:
  - Make paste-last deterministic by default.
  - Preserve adaptive option for users who prefer context-aware insertion.
  - Avoid regressions in transcribe/refine/undo flows.
- Non-Goals:
  - Any smart insertion algorithm changes.
  - Any database changes.

## Decisions
- Decision: Introduce `PastePreparationMode` (`Adaptive`, `Literal`) in shared paste utility.
  - Rationale: Keep one paste implementation while allowing per-flow preparation policy.
- Decision: Default paste-last to `Literal`.
  - Rationale: Matches user expectation of replaying exactly what History shows.
- Decision: Expose setting in Advanced near paste settings.
  - Rationale: Discoverable and consistent with insertion behavior controls.

## Risks / Trade-offs
- Risk: Some users may expect previous adaptive behavior.
  - Mitigation: Explicit opt-in toggle and clear copy.
- Risk: Divergence between flows could confuse QA.
  - Mitigation: Update manual checklist to reflect intentional difference.

## Migration Plan
- Additive settings-only migration via serde default (`false`) for missing field.
- Existing users retain data and automatically get deterministic default.

## Validation Plan
- `cargo test`
- `bun run test`
- `bun run check:translations`
- `openspec validate update-paste-last-transcript-format-mode --strict`
- `openspec validate --all --strict`
