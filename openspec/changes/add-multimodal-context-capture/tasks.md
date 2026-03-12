## 1. Multimodal Domain Model and Session State
- [ ] 1.1 Add multimodal domain types for `ActiveContext`, `CapturedFigure`, and `MultimodalSidecar`, including a stable serialized shape with `payload_version`.
- [ ] 1.2 Add a payload renderer that emits the exact contract:
  - optional `Context:` block,
  - non-empty context fields in fixed order `App`, `Window`, `URL`, `Selection`,
  - spoken or refined text with inline `[Figure N]`,
  - footer lines `Figure N: <absolute path>`.
- [ ] 1.3 Add runtime session state for active multimodal dictation:
  - multimodal-enabled flag snapshot,
  - captured `ActiveContext`,
  - session figure directory,
  - in-memory figure list with sequence order,
  - at-most-one in-flight interactive screenshot process,
  - screenshot count for overlay/UI updates.
- [ ] 1.4 Integrate active-context capture into dictation session start only when multimodal capture is enabled, reusing the existing AX/AppleScript/clipboard fallback chain.
- [ ] 1.5 Ensure multimodal capture is bypassed cleanly for voice-only sessions and when the setting is disabled, without changing current dictation behavior.

## 2. Settings, Shortcut Registration, and Frontend Wiring
- [ ] 2.1 Add the multimodal capture setting to persisted app settings with defaults and backward-compatible load behavior for existing users.
- [ ] 2.2 Add the new shortcut binding `capture_dictation_screenshot` with macOS default `control+command+s`, default-binding sync behavior, and reserved-shortcut validation.
- [ ] 2.3 Register the new binding in the backend shortcut/action system and ensure it is exposed through generated Tauri bindings instead of raw `invoke`.
- [ ] 2.4 Wire the new setting and binding through the frontend settings store, command updaters, and shortcut-recording flow.
- [ ] 2.5 Expose the new shortcut in the Keyboard Shortcuts UI with localized name and description text.
- [ ] 2.6 Add localized settings text explaining multimodal capture, hands-free-only screenshot support, and likely Accessibility, Automation, and Screen Recording prompts.

## 3. Screenshot Capture Execution Flow
- [ ] 3.1 Implement the screenshot action using `/usr/sbin/screencapture -i -o -x <session-path>` and allow it only while an active hands-free dictation session is running.
- [ ] 3.2 Record `captured_at` when the screenshot action is accepted, not when the file appears on disk, so alignment follows user intent.
- [ ] 3.3 Preserve Apple’s native interactive behavior across multi-monitor setups by not restricting capture to a single display and by allowing both region and window selection flows.
- [ ] 3.4 Serialize screenshot requests so only one interactive screenshot process can be in flight per session; ignore extra triggers while capture is already active.
- [ ] 3.5 Treat these outcomes as non-fatal no-figure results:
  - user cancels interactive capture,
  - process exits without a file,
  - interactive capture is redirected to the clipboard,
  - Screen Recording permission denial.
- [ ] 3.6 If dictation stop is requested while a screenshot is still in progress, wait for that in-flight screenshot to resolve before final dispatch, then attach only a successfully created file.
- [ ] 3.7 Hide transient Codictate capture UI, including the recording overlay, while interactive screenshot capture is active and restore it afterward.
- [ ] 3.8 Emit or update runtime screenshot-count state so the overlay and related UI can reflect attached figures for the active session.

## 4. Session Storage and History Persistence
- [ ] 4.1 Store successful screenshots in an app-managed per-session directory under app data during active dictation.
- [ ] 4.2 On successful history save, promote session screenshots into stable history storage under an entry-scoped directory and update sidecar file paths to the promoted absolute paths.
- [ ] 4.3 Add a nullable history field for serialized `MultimodalSidecar` metadata without changing the meaning of `transcription_text`, `post_processed_text`, or `inserted_text`.
- [ ] 4.4 Keep `transcription_text` as spoken transcript text, `post_processed_text` as refined spoken text, and `inserted_text` as the exact replay payload.
- [ ] 4.5 Handle partial screenshot-promotion failures pragmatically:
  - do not fail the transcription save,
  - keep only successfully promoted figures in persisted sidecar data,
  - render the final payload from the persisted subset.
- [ ] 4.6 Wire screenshot cleanup into history entry deletion and retention pruning so orphaned promoted files are removed with their owning entry.
- [ ] 4.7 Ensure legacy rows with no multimodal sidecar remain fully valid and continue to use existing voice-only history behavior.

## 5. Alignment, Rendering, and Reinsertion
- [ ] 5.1 Add sentence-boundary alignment helpers that assign each figure a durable `anchor_sentence_ordinal`, using transcription timing when available and pragmatic fallbacks when it is not.
- [ ] 5.2 Persist figure ordering by capture sequence and preserve that order through rendering, replay, and reinsertion.
- [ ] 5.3 Re-render multimodal payloads from spoken text plus sidecar data instead of parsing previously rendered payload text.
- [ ] 5.4 After AI refinement or post-processing, split the new spoken text into sentence boundaries and reinsert figures in original order using stored sentence ordinals.
- [ ] 5.5 If a stored sentence ordinal no longer maps cleanly after text changes, append remaining figure markers at the end of the text body before the footer.
- [ ] 5.6 Ensure rendering omits the entire `Context:` block when all context fields are empty and omits the footer when no figures remain.

## 6. Paste, Replay, Undo, and AI Integration
- [ ] 6.1 Update the live transcription paste path so voice-only output keeps existing adaptive smart-insertion behavior, while multimodal sessions paste the rendered structured payload literally.
- [ ] 6.2 Add a provider-agnostic strip-and-reattach layer so all AI post-processing paths operate on spoken text only and never receive the context block, inline figure markers, footer paths, or raw sidecar metadata.
- [ ] 6.3 Apply that strip-and-reattach behavior consistently to:
  - live post-processing after transcription,
  - manual `refine_last_transcript`,
  - repeated refine passes that start from `post_processed_text` when present.
- [ ] 6.4 Re-render the multimodal payload after AI returns and before paste/history update, using stored sidecar metadata rather than trusting model output to preserve figure paths.
- [ ] 6.5 Update `paste_last_transcript` so multimodal entries replay the exact original payload literally even when adaptive paste-last mode is enabled, while voice-only entries retain existing adaptive mode behavior.
- [ ] 6.6 Ensure undo capture continues to register the actual pasted payload returned by the shared paste utility for multimodal and voice-only flows alike.

## 7. History Search, Preview, Copy, and Replay Semantics
- [ ] 7.1 Keep History row previews compact by rendering spoken or refined transcript preview text rather than the full structured multimodal payload.
- [ ] 7.2 Add compact multimodal disclosure in History, such as badges or figure counts, without dumping file paths into the primary row text.
- [ ] 7.3 Update History copy behavior so it uses the exact replay payload when available and otherwise falls back to deterministic re-rendering or spoken preview text.
- [ ] 7.4 Update History search behavior so rows can match:
  - exact replay payload text,
  - spoken or refined preview text,
  - raw ASR text.
- [ ] 7.5 Ensure existing `effective_text` behavior for legacy rows remains unchanged and multimodal rows do not break current API compatibility.

## 8. UI Feedback and User Documentation
- [ ] 8.1 Add the multimodal capture toggle to Settings and ensure the backend fully bypasses multimodal capture work when the toggle is off.
- [ ] 8.2 Update the recording overlay and any related frontend state so the active session can show screenshot count and recover cleanly after screenshot UI hide/restore.
- [ ] 8.3 Add localized permission guidance for:
  - Accessibility and Automation for context capture,
  - Screen Recording for screenshot capture.
- [ ] 8.4 Write or update the multimodal feature guide to cover:
  - hands-free-only screenshot support in v1,
  - the dedicated screenshot shortcut,
  - file-backed screenshots only,
  - multi-monitor region/window capture support,
  - replay/refine behavior with figures,
  - what happens on cancel or missing permission.

## 9. Focused Verification
- [ ] 9.1 Add focused unit tests for payload rendering:
  - context block omission,
  - fixed field order,
  - inline figure numbering,
  - footer formatting,
  - no-figure and no-context fallbacks.
- [ ] 9.2 Add focused tests for sentence-boundary reinsertion and fallback-to-end-of-body behavior after refined text changes.
- [ ] 9.3 Add backend tests for screenshot capture outcomes:
  - success,
  - cancel,
  - missing file,
  - clipboard redirection,
  - Screen Recording denial,
  - duplicate trigger while capture is already in flight.
- [ ] 9.4 Add tests for stop-while-capturing behavior so final dispatch waits for the single in-flight screenshot and only attaches successful results.
- [ ] 9.5 Add tests for exact multimodal replay in `paste_last_transcript`, including the case where adaptive paste-last mode is enabled.
- [ ] 9.6 Add tests for provider-agnostic AI strip-and-reattach behavior across live post-process and `refine_last_transcript`.
- [ ] 9.7 Add tests proving History preview stays spoken-text-only while copy/replay use the exact payload and search still matches replay text plus raw text.
- [ ] 9.8 Add tests for screenshot cleanup during history deletion and retention pruning.
- [ ] 9.9 Perform targeted manual verification on macOS for:
  - region capture on a non-primary monitor,
  - window capture on a non-primary monitor,
  - overlay not appearing inside the captured screenshot,
  - denied Screen Recording permission guidance,
  - end-to-end multimodal dictation followed by refine and paste-last replay.
