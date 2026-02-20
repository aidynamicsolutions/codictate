## Context
Smart insertion is now profile-aware, but language coverage is still intentionally limited and partly hardcoded. This keeps behavior safe, yet it also means many valid languages remain in conservative mode for long periods. The next improvement must scale language coverage while preserving the existing UX safety bar.

## Goals / Non-Goals
- Goals:
  - Expand deterministic profile routing to most Whisper-supported languages with script-based grouping.
  - Keep conservative fallback behavior for ambiguous language/script cases.
  - Reduce ad-hoc boundary checks by moving toward Unicode-aware segmentation heuristics.
  - Establish robust quality controls with regression and golden tests.
  - Add low-cardinality telemetry signals to tune profile mapping safely over time.
- Non-Goals:
  - Rework accessibility context capture architecture.
  - Change core paste flow contracts (`paste`, `paste_last_transcript`, `refine_last_transcript`).
  - Introduce aggressive heuristics for uncertain languages.

## Decisions
- Decision: Use staged rollout (medium first, high second) with strict fallback guarantees.
  - Rationale: Coverage expansion is high-value, but punctuation/spacing regressions are UX-sensitive.

- Decision: Normalize selected language using canonical BCP47 handling and script inference.
  - Rationale: Locale variants (language/region/script) should resolve consistently to the same behavior class when linguistically equivalent for insertion rules.

- Decision: Use script-based grouping for most language routing.
  - Rationale: Script families are a practical proxy for spacing/casing behavior and scale better than one-off language allowlists.

- Decision: Move profile rules into data-driven configuration in the high-effort phase.
  - Rationale: Reduces Rust branching complexity and makes future language additions safer and reviewable.

- Decision: Add Unicode-aware boundary/token heuristics (word/sentence segmentation) in high-effort phase.
  - Rationale: Aligning heuristics with Unicode segmentation guidance improves behavior for multilingual text beyond ASCII-centric checks.

- Decision: Add telemetry as reason-coded, low-cardinality signals with no transcript payloads.
  - Rationale: Enables tuning and drift detection without collecting user content.

## Phase Plan
### Medium Effort (3-7 days): Broader Safe Coverage
- Expand language->profile mapping to most Whisper languages using script-group defaults.
- Keep conservative mode for unresolved/ambiguous tags and mixed-risk edge cases.
- Add regression tests per profile with representative languages and boundary cases.
- Keep existing fallback semantics and shared paste/undo behavior unchanged.

### High Effort (2-4 weeks): Robust Long-Term Engine
- Introduce versioned profile table data model (language/script/profile/punctuation classes/feature flags).
- Build config loader and validation with fail-closed conservative fallback.
- Upgrade to Unicode-aware word boundary/token heuristics and sentence-mark handling.
- Add golden fixtures and conformance-style boundary samples.
- Add decision telemetry and run tuning cycles based on aggregate fallback/decision patterns.

## Risks / Trade-offs
- Risk: Incorrect script inference for multi-script languages causes profile misclassification.
  - Mitigation: conservative fallback for ambiguous tags, explicit overrides, and test coverage for known multi-script languages.

- Risk: Unicode-aware segmentation adds runtime cost.
  - Mitigation: benchmark before rollout, cache reusable data structures, and gate expensive paths behind feature flags when needed.

- Risk: Data-driven config drift or malformed table entries.
  - Mitigation: strict schema validation and fail-closed fallback to conservative mode.

- Risk: Telemetry causes privacy or cardinality issues.
  - Mitigation: no transcript text in events, enumerated reason codes, and bounded attribute sets.

## Validation Plan
- Rust unit tests for normalization, profile resolution, spacing/casing decisions, punctuation sanitation, and fallback semantics.
- Representative multilingual regression matrix (at least one language per profile with punctuation and continuation variants).
- Golden fixtures for known edge-case transcripts and boundary interactions.
- Performance checks for boundary processing hot paths.
- `openspec validate update-smart-transcript-insertion-unicode-profile-engine --strict`
- `openspec validate --strict`

## External References
- Unicode Text Segmentation (UAX #29): https://www.unicode.org/reports/tr29/
- Unicode Script Property (UAX #24): https://www.unicode.org/reports/tr24/
- Unicode LDML/CLDR data model (TR35): https://unicode.org/reports/tr35/
- ICU boundary analysis overview: https://unicode-org.github.io/icu/userguide/boundaryanalysis/
- BCP47 language tags (RFC 5646): https://www.rfc-editor.org/rfc/rfc5646
- Language Subtag Registry (IANA): https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry
- Whisper tokenizer language inventory reference: https://raw.githubusercontent.com/openai/whisper/main/whisper/tokenizer.py
- Unicode boundary conformance test datasets (UCD auxiliary): https://www.unicode.org/Public/UCD/latest/ucd/auxiliary/
- OpenTelemetry semantic conventions (error/feature-flag signal design): https://opentelemetry.io/docs/specs/semconv/general/recording-errors/ and https://opentelemetry.io/docs/specs/semconv/feature-flags/
