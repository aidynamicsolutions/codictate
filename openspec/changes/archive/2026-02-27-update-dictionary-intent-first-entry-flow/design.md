## Context
Dictionary matching now follows a precision-first backend policy (exact + aliases default, fuzzy opt-in with short-target safety). The UI should reflect that model with a novice-friendly primary path and conditional fuzzy exposure only when relevant.

## Goals
- Make the first decision explicit: recognition vs replacement intent.
- Keep aliasing discoverable and easy.
- Keep fuzzy available but de-emphasized.
- Preserve existing `CustomWordEntry` payload contract.

## Non-Goals
- Build a Snippets entity, page, storage model, or routing.
- Change fuzzy thresholds or matching internals.

## UI State Model

| State | Context | Key Controls | Save Rule |
| --- | --- | --- | --- |
| S1 | Create + recognize (ineligible/empty input) | Intent selector, input, aliases | Save when input valid + non-duplicate |
| S2 | Create + replace | Intent selector, input (`What you say`), output text, aliases | Save when input valid + non-duplicate + output differs |
| S3 | Edit existing recognize | Hydrate recognize intent from entry | Preserve aliases and fuzzy unless blocked by guard |
| S4 | Edit existing replace | Hydrate replace intent from entry | Fuzzy hidden and forced off |
| S5 | Recognize + eligible input | Fuzzy toggle row + tooltip | Only save fuzzy true when eligible |

## Data Mapping
- `intent = recognize`
  - `is_replacement = false`
  - `replacement = input`
  - `fuzzy_enabled = toggle && !short_target_block`
- `intent = replace`
  - `is_replacement = true`
  - `replacement = output_text`
  - `fuzzy_enabled = false`

## Accessibility
- Intent selector is a keyboard-navigable radio group.
- Info icons keep short, single-sentence tooltips.
- Default view should show at most one toggle (hidden until recognize input is fuzzy-eligible).

## Rollout
- UI/copy change only; no migration needed.
- Validate via existing unit tests + new utility tests + manual QA matrix.
