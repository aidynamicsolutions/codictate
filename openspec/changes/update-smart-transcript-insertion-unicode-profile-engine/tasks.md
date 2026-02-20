## 1. OpenSpec Deltas
- [ ] 1.1 Add `transcript-insertion` delta for expanded profile coverage, Unicode boundary heuristics, data-driven rules, and telemetry requirements.
- [ ] 1.2 Validate with `openspec validate update-smart-transcript-insertion-unicode-profile-engine --strict`.

## 2. Medium Effort (3-7 days): Broader Safe Coverage
- [ ] 2.1 Add canonical language normalization updates for robust BCP47-style tag handling.
- [ ] 2.2 Expand smart insertion profile routing to most Whisper languages using script-based grouping.
- [ ] 2.3 Keep conservative fallback for unresolved, ambiguous, and explicitly high-risk language cases.
- [ ] 2.4 Add regression tests per profile using representative languages and punctuation/spacing continuation scenarios.
- [ ] 2.5 Update user-facing Smart Insertion copy and docs only where behavior changes need disclosure.

## 3. High Effort (2-4 weeks): Robust Long-Term Engine
- [ ] 3.1 Introduce a versioned data-driven smart insertion profile table.
- [ ] 3.2 Implement table loading and schema validation with fail-closed conservative fallback.
- [ ] 3.3 Replace ad-hoc boundary checks with Unicode-aware word/token/sentence boundary heuristics.
- [ ] 3.4 Add golden fixtures plus conformance-style Unicode boundary samples for regression protection.
- [ ] 3.5 Add low-cardinality reason-coded telemetry for profile resolution, fallback paths, and key heuristic decisions.
- [ ] 3.6 Run telemetry-informed tuning pass and document rule adjustments and rationale.

## 4. Verification
- [ ] 4.1 Run targeted Rust tests for `smart_insertion` and shared paste pathways.
- [ ] 4.2 Confirm no regression in live paste, paste-last, refine-last, and undo payload behavior.
- [ ] 4.3 Run `openspec validate --strict`.
