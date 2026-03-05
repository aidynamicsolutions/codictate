## 1. Docs and Copy Alignment (Phase 0)
- [x] 1.1 Update Dictionary user guide to clarify post-ASR correction behavior and limits
- [x] 1.2 Add ambiguous-word safety guidance and transcript-first mispronunciation workflow
- [x] 1.3 Update technical custom-word correction guide to align guarantees and policy
- [x] 1.4 Update English in-app microcopy to avoid guaranteed pronunciation-bias wording

## 2. Data Model and Persistence (Phase 1)
- [ ] 2.1 Extend `CustomWordEntry` with optional `context_scope` (`prev_any`, `next_any`, `window_any`)
- [ ] 2.2 Add optional `ambiguity_level` metadata (`low` | `high`) for UX safety messaging
- [ ] 2.3 Keep serialization/deserialization backward compatible for entries without scope
- [ ] 2.4 Confirm no required migration for existing `user_dictionary.json` entries

## 3. Matcher Behavior (Phase 1)
- [ ] 3.1 Implement scope-gated exact candidate acceptance
- [ ] 3.2 Enforce matching precedence: phrase exact > scoped exact > unscoped exact > fuzzy
- [ ] 3.3 Keep existing fuzzy guardrails unchanged
- [ ] 3.4 Add reason-coded diagnostics for scoped skips and scoped accepts

## 4. UX Safety and Guidance (Phase 1)
- [ ] 4.1 Add ambiguous-alias warning surface using `ambiguity_level`
- [ ] 4.2 Recommend phrase replacement or scoped rules for high-ambiguity single-word aliases
- [ ] 4.3 Preserve current behavior when scope is absent

## 5. Validation and Tests (Phase 1)
- [ ] 5.1 Scoped positive: `review the state changes` -> `review the staged changes`
- [ ] 5.2 Scoped negative: `I want the state change` remains unchanged
- [ ] 5.3 Phrase precedence: `state changes` phrase rule overrides generic alias behavior
- [ ] 5.4 Backward compatibility: legacy entries without scope behave exactly as current
- [ ] 5.5 Safety regression: existing guards (for example `mode -> modal`) still prevent overfire
