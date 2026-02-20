# Smart Insertion Language Profiles Manual Test Checklist

## Purpose
Validate the smart insertion language-profile logic, including conservative fallback spacing (no extra space before punctuation), internal-space compaction for Chinese/Cantonese/Japanese, and no regressions across paste flows.

## Legend
- `|` = cursor position marker (do **not** type this character).
- `[text]` = currently selected text marker (do **not** type `[` or `]`).
- `Paste trigger` can be any of: `transcribe`, `paste_last_transcript`, `refine_last_transcript`.
- Use a plain text editor (TextEdit/Notes) to avoid editor-specific auto-format behavior.
- `Seed text (before paste)` means the exact text that should already be in your editor/input field before you trigger paste.
- `Transcript to paste` means the exact output text that Codictate should insert.
- In all seed examples, markers are instructions only:
  - remove `|` before typing,
  - remove `[` and `]` before typing,
  - then place cursor/select text exactly where markers indicate.

## Marker Conversion Examples (Important)
- Notation: `Hello |, world`
  - Type in editor: `Hello , world`
  - Then place cursor between `Hello` and `,`.
- Notation: `start [MIDDLE]step`
  - Type in editor: `start MIDDLEstep`
  - Then select only `MIDDLE`.

## How To Run One Test Row
1. Set `Language` and `Smart Insertion` exactly as the row specifies.
2. Clear the editor, then type the seed text **without marker characters** (`|`, `[`, `]`).
3. Place the cursor where `|` appears.
4. If the seed uses `[text]`, select that exact text before triggering paste.
5. Trigger paste using the row's `Paste trigger`.
6. Use the row's `Transcript to paste` phrase.
7. Compare the final editor text to `Expected result` exactly.
8. If the result differs, mark FAIL and record row ID + actual output.

## Example (SMK-01)
- Row says seed: `Hello |, world`.
- You type `Hello , world`.
- Place cursor between `Hello` and `,`.
- Trigger paste with transcript `there`.
- Expected final text: `Hello there, world`.

## One-Time Setup
- [ ] Step 1: Launch Codictate and a plain text editor.
- [ ] Step 2: Open settings and enable `Smart Insertion`.
- [ ] Step 3: Ensure accessibility permission is granted (for context-aware insertion tests).
- [ ] Step 4: Keep this checklist open and copy each seed string exactly.
- [ ] Step 5: For each test case, set language exactly as requested in the `Language` column.
- [ ] Step 6: Use the same target app for all tests (avoid mixing editors during one run).
- [ ] Step 7: Run rows top-to-bottom for fastest isolation when something fails.

## Quick Smoke (must pass first)
| Done | ID | Language | Smart Insertion | Seed text (before paste) | Transcript to paste | Paste trigger | Expected result |
|---|---|---|---|---|---|---|---|
| [ ] | SMK-01 | `Auto` | ON | `Hello |, world` | `there` | transcribe | `Hello there, world` (no space before comma) |
| [ ] | SMK-02 | `Auto` | ON | `Hello |world` | `new` | transcribe | `Hello new world` |
| [ ] | SMK-03 | `English` | ON | `. |` | `hello` | transcribe | `. Hello` |
| [ ] | SMK-04 | `Chinese` | ON | `你|好` | `世界` | transcribe | `你世界好` (no inserted spaces) |
| [ ] | SMK-05 | `Chinese` | ON | `你|好` | `是 請` | transcribe | `你是請好` (internal ASR space compacted) |
| [ ] | SMK-06 | `Cantonese` | ON | `你|好` | `係 唔係` | transcribe | `你係唔係好` (internal ASR space compacted) |

## Conservative Profile (`Auto`, `Turkish`, unsupported) Cases
These cases validate the new conservative behavior: boundary-safe trailing space with no pre-punctuation spaces.

| Done | ID | Language | Smart Insertion | Seed text (before paste) | Transcript to paste | Paste trigger | Expected result |
|---|---|---|---|---|---|---|---|
| [ ] | CON-01 | `Auto` | ON | `Hi |, there` | `team` | transcribe | `Hi team, there` |
| [ ] | CON-02 | `Auto` | ON | `Hi |. there` | `team` | transcribe | `Hi team. there` |
| [ ] | CON-03 | `Auto` | ON | `Hi |? there` | `team` | transcribe | `Hi team? there` |
| [ ] | CON-04 | `Auto` | ON | `Hi |! there` | `team` | transcribe | `Hi team! there` |
| [ ] | CON-05 | `Auto` | ON | `Hi |there` | `team` | transcribe | `Hi team there` |
| [ ] | CON-06 | `Auto` | ON | `Hi |there` | `team ` | transcribe | `Hi team there` (no double space) |
| [ ] | CON-07 | `Auto` | ON | `Hi |there` | `team!` | transcribe | `Hi team!there` (no forced trailing space after non-word ending) |
| [ ] | CON-08 | `Turkish` | ON | `Merhaba |, dünya` | `takım` | transcribe | `Merhaba takım, dünya` |
| [ ] | CON-09 | `Turkish` | ON | `Merhaba |dünya` | `takım` | transcribe | `Merhaba takım dünya` |
| [ ] | CON-10 | `Auto` | OFF | `Hi |, there` | `team` | transcribe | Raw paste behavior; no smart insertion adjustments |

## Cased Whitespace Profile Cases (`English`, `Spanish`, `French`, `German`, `Italian`, `Portuguese`, `Polish`, `Czech`, `Russian`, `Ukrainian`, `Vietnamese`)
These punctuation conflict cleanups apply only to whitespace profiles. Conservative mode (`Auto`/`Turkish`/unsupported) keeps minimal edits.

| Done | ID | Language | Smart Insertion | Seed text (before paste) | Transcript to paste | Paste trigger | Expected result |
|---|---|---|---|---|---|---|---|
| [ ] | CWS-01 | `English` | ON | `. |` | `hello` | transcribe | `. Hello` (capitalize sentence start) |
| [ ] | CWS-02 | `English` | ON | `? |` | `hello` | transcribe | `? Hello` |
| [ ] | CWS-03 | `English` | ON | `word |` | `Title` | transcribe | `word title` (decapitalize title-like mid-sentence) |
| [ ] | CWS-04 | `English` | ON | `word |` | `NASA` | transcribe | `word NASA` (acronym preserved) |
| [ ] | CWS-05 | `English` | ON | `start |step` | `hello?` | transcribe | `start hello step` (`?` stripped before lowercase continuation) |
| [ ] | CWS-06 | `English` | ON | `start |٣` | `hello.` | transcribe | `start hello ٣` (`.` stripped before Unicode numeric continuation; word boundary spaced) |
| [ ] | CWS-07 | `English` | ON | `start |Step` | `hello?` | transcribe | `start hello? Step` (`?` preserved and spaced before uppercase continuation) |
| [ ] | CWS-08 | `English` | ON | `start |step` | `e.g.` | transcribe | `start e.g. step` (abbreviation guard preserved; trailing sentence-boundary space added; no `e.g..` duplication when dictionary alias maps `e g` -> `e.g.`) |
| [ ] | CWS-09 | `English` | ON | `start |.` | `hello.` | transcribe | `start hello.` (duplicate boundary period collapsed) |
| [ ] | CWS-14 | `English` | ON | `start |.` | `hello?` | transcribe | `start hello.` (conflicting sentence punctuation at boundary prefers existing right-boundary mark) |
| [ ] | CWS-15 | `English` | ON | `Hello |, world` | `there.` | transcribe | `Hello there, world` (sentence punctuation before clause punctuation is cleaned up) |
| [ ] | CWS-16 | `English` | ON | `start |, step` | `hello?` | transcribe | `start hello, step` (question mark before comma is cleaned up) |
| [ ] | CWS-17 | `English` | ON | `start |, step` | `e.g.` | transcribe | `start e.g., step` (abbreviation period is preserved before comma) |
| [ ] | CWS-10 | `English` | ON | `. |next` | `what` | transcribe | `. What next` (leading and trailing word-boundary spaces) |
| [ ] | CWS-11 | `English` | ON | `word|next` | `,` | transcribe | `word,next` (no extra space before punctuation insertion) |
| [ ] | CWS-12 | `Spanish` | ON | `. |` | `hola` | transcribe | `. Hola` (same cased profile behavior) |
| [ ] | CWS-13 | `English` | ON | `word|next` | `?` | transcribe | `word?next` (punctuation-only insertion does not force trailing boundary space) |

## Selection Replacement Cases (context `has_selection = true`)
Use a real text selection, then paste to replace it.

| Done | ID | Language | Smart Insertion | Seed text (before paste) | Transcript to paste | Paste trigger | Expected result |
|---|---|---|---|---|---|---|---|
| [ ] | SEL-01 | `English` | ON | `start [MIDDLE]step` (select `MIDDLE`) | `hello?` | transcribe | `start hello step` (`?` stripped with lowercase continuation after selection) |
| [ ] | SEL-02 | `English` | ON | `start [MIDDLE]Step` (select `MIDDLE`) | `hello?` | transcribe | `start hello? Step` (`?` preserved with sentence-boundary spacing before uppercase continuation) |

## Uncased Whitespace Profile Cases (`Korean`, `Arabic`, `Persian`, `Urdu`, `Hebrew`)

| Done | ID | Language | Smart Insertion | Seed text (before paste) | Transcript to paste | Paste trigger | Expected result |
|---|---|---|---|---|---|---|---|
| [ ] | UWS-01 | `Korean` | ON | `. |` | `word` | transcribe | `. word` (no forced capitalization) |
| [ ] | UWS-02 | `Korean` | ON | `prev |` | `Title` | transcribe | `prev Title` (no forced decapitalization) |
| [ ] | UWS-03 | `Arabic` | ON | `مرحبا |سلام` | `كيف؟` | transcribe | `مرحبا كيف سلام` (Arabic `؟` stripped before alphabetic continuation) |
| [ ] | UWS-04 | `Arabic` | ON | `مرحبا |سلام` | `e.g؟` | transcribe | `مرحبا e.g؟ سلام` (abbreviation guard preserved; sentence-boundary spacing added) |
| [ ] | UWS-07 | `Arabic` | ON | `مرحبا |.` | `سلام؟` | transcribe | `مرحبا سلام.` (conflicting sentence punctuation at boundary prefers existing right-boundary mark) |
| [ ] | UWS-08 | `Arabic` | ON | `مرحبا |، عالم` | `كيف؟` | transcribe | `مرحبا كيف، عالم` (question mark before Arabic comma is cleaned up) |
| [ ] | UWS-05 | `Korean` | ON | `안녕|세상` | `친구` | transcribe | `안녕 친구 세상` (word-boundary spacing allowed) |
| [ ] | UWS-06 | `Korean` | ON | `안녕|세상` | `,` | transcribe | `안녕,세상` (no extra spacing around punctuation token) |

## No-Boundary-Spacing Profile Cases (`Chinese`, `Chinese Traditional`, `Cantonese`, `Japanese`, `Thai`, `Khmer`, `Lao`, `Burmese`, `Tibetan`)

| Done | ID | Language | Smart Insertion | Seed text (before paste) | Transcript to paste | Paste trigger | Expected result |
|---|---|---|---|---|---|---|---|
| [ ] | NBS-01 | `Chinese` | ON | `你|好` | `世界` | transcribe | `你世界好` (no boundary spaces) |
| [ ] | NBS-02 | `Chinese` | ON | `。|好` | `世界` | transcribe | `。世界好` (no forced leading space after `。`) |
| [ ] | NBS-03 | `Chinese` | ON | `你|好` | `世界。` | transcribe | `你世界好` (terminal `。` stripped before alphabetic continuation; still no boundary spaces) |
| [ ] | NBS-04 | `Chinese` | ON | `你好|。` | `世界。` | transcribe | `你好世界。` (duplicate `。` collapsed to one) |
| [ ] | NBS-05 | `Chinese` | ON | `你好|字` | `U.S.A。` | transcribe | `你好U.S.A。字` (abbreviation-like token preserved) |
| [ ] | NBS-06 | `Japanese` | ON | `こ|れ` | `テスト` | transcribe | `こテストれ` (no auto spaces) |
| [ ] | NBS-07 | `Chinese` | ON | `你|好` | `是 請` | transcribe | `你是請好` (compact Han/Han internal spaces) |
| [ ] | NBS-08 | `Chinese` | ON | `你好。|再见` | `谢谢` | transcribe | `你好。谢谢再见` (no inserted space after sentence punctuation) |
| [ ] | NBS-09 | `Cantonese` | ON | `你|好` | `係 唔係` | transcribe | `你係唔係好` (compact Han/Han internal spaces) |
| [ ] | NBS-10 | `Chinese` | ON | `你|好` | `Open AI` | transcribe | `你Open AI好` (preserve intentional ASCII phrase spacing) |
| [ ] | NBS-11 | `Japanese` | ON | `こ|れ` | `私 は コーヒー を 飲みました。` | transcribe | `こ私はコーヒーを飲みました。れ` (compact Japanese internal spaces) |
| [ ] | NBS-12 | `Japanese` | ON | `今|日` | `iPhone 16 を 買った` | transcribe | `今iPhone 16を買った日` (compact ASCII↔Japanese boundaries; preserve `ASCII↔ASCII`) |
| [ ] | NBS-13 | `Japanese` | ON | `今|日` | `Open AI の API` | transcribe | `今Open AIのAPI日` (preserve `Open AI`, compact around Japanese particle) |
| [ ] | NBS-14 | `Japanese` | ON | `今|日` | Two-line transcript: first line `私は`, second line `コーヒー` | transcribe | Two-line output preserved as `今私は` then `コーヒー日` (line break is not compacted) |
| [ ] | NBS-15 | `Japanese` | ON | `今|日` | `了解 👍 です` | transcribe | `今了解 👍 です日` (emoji-adjacent spacing is preserved) |

## Language Normalization Routing Cases
Use these to validate BCP47 tag normalization behavior when those variants are available in your language picker.

| Done | ID | Language selection | Smart Insertion | Seed text (before paste) | Transcript to paste | Expected result |
|---|---|---|---|---|---|---|
| [ ] | LNG-01 | `English (US)` / `en-US` | ON | `. |` | `hello` | Cased behavior (`. Hello`) |
| [ ] | LNG-02 | `Portuguese (Brazil)` / `pt-BR` | ON | `. |` | `ola` | Cased behavior (`. Ola`) |
| [ ] | LNG-03 | `Chinese (Simplified)` / `zh-Hans` | ON | `你|好` | `世界` | No-boundary-spacing behavior |
| [ ] | LNG-04 | `Chinese (Traditional)` / `zh-TW` or `zh-Hant` | ON | `你|好` | `世界` | No-boundary-spacing behavior |
| [ ] | LNG-05 | `Cantonese` / `yue-*` | ON | `你|好` | `世界` | No-boundary-spacing behavior |
| [ ] | LNG-06 | `Auto` | ON | `Hi |, there` | `team` | Conservative behavior (no pre-punctuation space) |
| [ ] | LNG-07 | `Turkish` / `tr` | ON | `Merhaba |, dünya` | `takım` | Conservative behavior |

## Shared Paste Flow Checks
Verify each flow with its intended behavior. `paste_last_transcript` now defaults to deterministic literal replay and no longer has to match transcribe/refine output in every context.

Flow prerequisites (important):
- Before `FLOW-01` to `FLOW-03`, disable auto-refine/post-processing so history text is stable.
- Prime history with a fresh transcript where the primary/effective text is exactly `there`.
- For `FLOW-03`, enable `Advanced -> Adapt Paste Last to Cursor`.

| Done | ID | Language | Scenario | Trigger | Expected result |
|---|---|---|---|---|---|
| [ ] | FLOW-01 | `English` | Seed `. |`, transcript `there` | `transcribe` | `. There` (adaptive smart insertion still applies) |
| [ ] | FLOW-02 | `English` | Seed `. |`, latest history primary is `there` | `paste_last_transcript` (default setting) | `. there` (literal replay, no forced capitalization) |
| [ ] | FLOW-03 | `English` | Seed `. |`, latest history primary is `there` | `paste_last_transcript` (adaptive setting ON) | `. There` (adaptive mode applies sentence-start capitalization) |
| [ ] | FLOW-04 | `English` | Seed `. |`, latest row raw text is `there` | `refine_last_transcript` (pass-through refine prompt) | `. There` (refine path remains adaptive) |

## History Inserted-Text Parity
Validate that History primary content reflects what was inserted into the target app, while raw ASR remains available for verification.

| Done | ID | Language | Scenario | Action | Expected result |
|---|---|---|---|---|---|
| [ ] | HST-01 | `English` | Boundary punctuation changes final inserted text (`Hello |, world` + transcript `there.`) | Trigger `transcribe`, then open History | History primary text matches inserted result (`there,` boundary behavior reflected in full line), not the raw ASR token with trailing period. |
| [ ] | HST-02 | `English` | Raw and inserted differ | In the same row, click `Original transcript` | Inline panel expands and shows raw ASR text; primary line stays unchanged. |
| [ ] | HST-03 | `English` | Search match exists only in raw | Search by a raw-only token while row is collapsed | Row appears in results and shows hint `Matched in original transcript`; click hint expands raw panel. |
| [ ] | HST-04 | `English` | Refine-last updates same row | Run `refine_last_transcript` on latest entry, then reopen History | Same latest row updates with refined primary text; raw panel still shows original ASR for that recording. |

## Optional Advanced Cases
Use these only if you want extra confidence beyond standard UI paths.

| Done | ID | Setup | Action | Expected result |
|---|---|---|---|---|
| [ ] | ADV-01 | Force context unavailable (for example, temporarily revoke accessibility permission and restart app) | Run `Auto` case with seed `Hello |world`, transcript `new` | Legacy fallback behavior appears (`new ` style trailing-space fallback when context is unavailable) |
| [ ] | ADV-02 | Configure an unsupported language code in settings (if your build/test tools allow this) | Repeat `Auto` conservative cases | Same conservative behavior as `Auto`/`Turkish` |

## Optional Debug Log Verification
If you run with debug logs, verify smart insertion reasons are emitted as expected.

- [ ] Step L1: Locate latest log file: `LOG_FILE=$(ls -1t ~/Library/Logs/com.pais.codictate/codictate*.log | head -n 1)`.
- [ ] Step L2: Inspect smart insertion events: `rg -n "Applied smart transcript insertion formatting|Applied conservative smart insertion fallback|insertion_profile|conservative_spacing_reason|duplicate_punctuation_collapse_reason|internal_space_compaction_applied|internal_space_compaction_reason|internal_space_compaction_removed_chars|fallback_mode" "$LOG_FILE" | tail -n 300`.
- [ ] Step L3: Confirm conservative no-punctuation case logs `conservative_spacing_reason=no_trailing_space_needed`.
- [ ] Step L4: Confirm conservative between-words case logs `conservative_spacing_reason=word_boundary_trailing_space`.
- [ ] Step L5: Confirm no-context case logs `conservative_spacing_reason=legacy_no_context`.
- [ ] Step L6: For `zh`/`zh-tw`/`yue`/`ja` internal-space cases, confirm logs show `internal_space_compaction_applied=true` and a positive `internal_space_compaction_removed_chars`.
- [ ] Step L7: For clause-boundary cleanup case (`CWS-15`/`CWS-16`), confirm logs show `duplicate_punctuation_collapse_reason=conflicting_clause_boundary_mark_prefer_right_boundary`.
- [ ] Step L8: For abbreviation guard case (`CWS-17`), confirm logs show `duplicate_punctuation_collapse_reason=clause_boundary_abbreviation_guard`.

## Final Pass Criteria
- [ ] Conservative mode never inserts a space before punctuation.
- [ ] Conservative mode still separates words when right boundary is word-like.
- [ ] Cased/uncased/no-boundary behaviors still match profile expectations.
- [ ] When sentence punctuation conflicts at a cursor boundary in whitespace profiles, Codictate keeps the existing right-boundary punctuation.
- [ ] In whitespace profiles, sentence punctuation before clause punctuation is cleaned up (for example `there.,` -> `there,`) while abbreviation periods remain intact (`e.g.,`).
- [ ] Japanese dictation output does not keep ASR artifact spaces at Japanese boundaries (`Japanese↔Japanese`, `ASCII↔Japanese`) while preserving intentional `ASCII↔ASCII` spacing.
- [ ] `paste_last_transcript` is deterministic/literal by default, and matches adaptive smart insertion behavior only when `Adapt Paste Last to Cursor` is enabled.
- [ ] If any test fails, record ID, observed output, expected output, app version/commit, and timestamp.
