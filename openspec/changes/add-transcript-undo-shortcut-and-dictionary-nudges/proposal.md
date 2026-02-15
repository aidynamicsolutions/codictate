# Change: Add Transcript Undo Shortcut and Dictionary Nudges

## Why
Users can misspeak, panic-correct, or otherwise reject a pasted transcript immediately after dictation. Today they must manually select and delete pasted text, which breaks the fast "speak -> check -> retry" loop.

The app already suggests dictionary aliases from History, but users must navigate there manually. Repeated undo behavior is a strong signal that the same recognition pattern is failing and should trigger a proactive suggestion.

## What Changes
- Add a new global shortcut action (`undo_last_transcript`) that undoes only the most recent tracked Handy-originated paste.
- Expose `undo_last_transcript` in shortcut settings with platform defaults and reset-to-default coverage.
- Add explicit paste-tracking state for strict undo:
  - single in-memory slot (most recent paste only)
  - tracked fields: `paste_id`, `source_action`, `auto_refined`, `pasted_text`, `suggestion_text`, timestamps, and consumed flag
  - tracked source actions include `transcribe`, `transcribe_with_post_process`, `paste_last_transcript`, and `refine_last_transcript`
  - TTL for "recent paste": 120 seconds
  - consumed after one undo dispatch attempt (second press is no-op unless a new Handy paste occurs)
- Define explicit no-op feedback paths (not silent):
  - no tracked slot: show `Nothing to undo`
  - expired slot: show `Undo expired`
  - recording active with no valid slot after cancellation: show `Recording canceled`
- Define processing-in-flight undo behavior:
  - pressing undo during active recording/transcription/refine/post-process triggers the same cancellation path as Escape
  - if a valid tracked recent paste exists, proceed with normal undo dispatch for that slot
  - if no valid tracked recent paste exists, stop the active operation and show `Processing canceled`
  - include a brief stop-transition grace marker (500ms) after recording stop so immediate undo can still cancel pending pipeline start
- Lock down text-capture semantics for suggestion evaluation:
  - `suggestion_text` is always the raw ASR transcript source (`transcription_text`), not post-refined output
  - `pasted_text` is the exact paste payload (including transport modifiers such as trailing space)
  - post-processed flows preserve originating `source_action` (`transcribe` or `transcribe_with_post_process`) with `auto_refined=true`
- Reuse existing alias-suggestion heuristics by adding a Rust<->frontend evaluation bridge:
  - backend emits evaluation requests with `paste_id` and `suggestion_text`
  - frontend runs `suggestAliasFromTranscript`
  - frontend returns candidate/no-candidate payload to backend
  - backend queues requests when evaluator is unavailable and flushes on evaluator-ready signal
- Deliver undo nudges with existing overlay UI first (interactive overlay card), with Linux-specific native notification bridge when overlay is unavailable by default.
- Add overlay CTA flow:
  - alias nudge: one-tap `Add "<alias>" -> "<term>"`
  - alias nudge includes suppression action `Don't suggest this` for that identity key
  - unresolved nudge: `Open Dictionary` with phrase excerpt context
  - overlay action to open Dictionary in main window (focus app + navigate section)
  - overlay controls include accessibility labeling, live announcements, and keyboard activation support
- Keep fallback when overlay UI is unavailable:
  - non-Linux: if main window is focused, show in-app toast immediately
  - Linux: show native notification first; notification activation opens/focuses app; then surface actionable in-app toast
- Use count-based nudge gating with no time cooldown:
  - alias nudge shown when matching evidence count is greater than 3
  - repeat alias nudge shown only after more than 3 additional matching events since last nudge
  - suppressed identities (`Don't suggest this`) are never surfaced again unless manually cleared in settings
- Add fallback behavior when no dictionary candidate is found:
  - accumulate unresolved undo evidence
  - show unresolved nudge after unresolved count is greater than 3.
- Add deterministic discoverability hint timing:
  - one-time-ever hint shown only after second successful tracked paste
  - 2.5 second delay before rendering
  - hint copy includes TTL clarity (`within 2 minutes`)
  - hint is skipped if user has already used undo once
- Persist undo-evidence state in a dedicated store file (`undo_nudge_store.json`) instead of `settings_store.json`.
- Add structured logging requirements for slot lifecycle, undo dispatch outcomes, heuristic bridge state, and nudge decisions.
- Document clipboard interaction trade-off: undo removes pasted text but does not restore clipboard beyond the configured paste mode semantics.
- Add documentation deliverable in `doc/` describing undo-last-transcript behavior, defaults, constraints, and Linux notification flow.

## Default Shortcut Bindings
- `undo_last_transcript` defaults:
  - macOS: `control+command+z`
  - Windows: `ctrl+alt+z`
  - Linux: `ctrl+alt+z`

## Impact
- **Affected specs**:
  - `shortcut-settings`
  - `custom-word-correction`
  - `observability`
- **Affected code** (planned):
  - `src-tauri/src/settings.rs`
  - `src-tauri/src/actions.rs`
  - `src-tauri/src/input.rs`
  - `src-tauri/src/shortcut/mod.rs`
  - `src-tauri/src/shortcut/reserved.rs`
  - `src-tauri/src/overlay.rs`
  - `src-tauri/src/notification.rs`
  - `src/overlay/RecordingOverlay.tsx`
  - `src/overlay/RecordingOverlay.css`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/window.rs`
  - `src/App.tsx`
  - `src/components/shared/KeyboardShortcutsModal.tsx`
  - `src/components/shared/HistoryList.tsx`
  - `src/utils/dictionaryAliasSuggestion.ts`
  - `src/stores/settingsStore.ts`
  - `src/bindings.ts`
  - `src/i18n/locales/*/translation.json`
  - `doc/undo-paste-last-transcript.md`
- **Behavioral impact**:
  - Users can quickly undo the last pasted transcript without manual text selection.
  - Undo shortcut is strict: no undo command is sent when there is no tracked recent Handy paste.
  - Undo shortcut dispatches one undo command only, and only within 120 seconds of tracked paste.
  - No-op undo presses always return user-visible feedback.
  - Users who repeatedly undo similar transcript output receive proactive dictionary guidance in overlay.
  - Dictionary nudges are available proactively from undo behavior, including unresolved fallback nudges.
