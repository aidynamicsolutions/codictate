## Context
The original proposal attempted to infer screenshot intent by listening for system screenshot shortcuts and watching the filesystem for newly created files. That approach creates avoidable ambiguity:
- screenshot completion is delayed and format-dependent,
- Desktop/custom-folder watching adds privacy and file access complexity,
- `Shift-Command-5` introduces a separate UI state machine,
- exact replay and AI-safe reinsertion become harder to specify.

This change keeps the existing active-context capture goal, but narrows screenshot capture to an app-owned path that Codictate can time, store, and replay deterministically.

## Goals / Non-Goals
- Goals:
  - Preserve a strong dictation UX for multimodal prompting.
  - Keep the pasted payload stable and human-readable.
  - Ensure AI post-processing and refine operate on spoken transcript text only.
  - Preserve exact replay for `paste_last_transcript`.
  - Keep History rows compact while retaining multimodal metadata.
- Non-Goals:
  - Passive tracking of arbitrary macOS screenshots or screenshot folders.
  - Support for `Cmd+Ctrl+Shift+4` clipboard screenshots in v1.
  - Push-to-talk screenshot capture in v1.
  - Parsing rendered text to recover screenshot metadata.

## Decisions
### 1. Payload contract
The rendered payload is:

```text
Context:
App: <app name>
Window: <window title>
URL: <url>
Selection: <selected text>

<spoken or refined text with inline [Figure N] tokens>

Figure 1: <absolute path>
Figure 2: <absolute path>
```

Rules:
- Emit `Context:` only when at least one context field is non-empty.
- Emit context fields in fixed order: `App`, `Window`, `URL`, `Selection`.
- Omit empty context fields.
- Emit the figure footer only when at least one figure exists.
- Preserve figure numbering and order from capture order.

### 2. Screenshot capture architecture
- Multimodal screenshots are captured only while an active hands-free dictation session is running.
- A dedicated binding `capture_dictation_screenshot` triggers `/usr/sbin/screencapture -i -o -x <session-path>`.
- The interactive capture intentionally does not pass `-m` or `-D`, so Apple’s native selection and window-picking UX continues to work across any attached display in multi-monitor setups.
- The command runs outside the paste/render path and succeeds only if:
  - `screencapture` exits successfully, and
  - the target output file exists afterward.
- `captured_at` is recorded when the screenshot action is accepted, not when the output file appears on disk, so alignment follows user intent rather than delayed file creation.
- Only one interactive screenshot process may be in flight per active dictation session. Additional screenshot shortcut presses while the first capture is active are ignored.
- While interactive screenshot UI is active, transient Codictate capture surfaces such as the recording overlay are hidden and then restored after the screenshot resolves, preventing accidental self-capture.
- If dictation stop is requested while a screenshot is still in progress, final dispatch waits for that in-flight screenshot to resolve. This replaces the earlier passive “grace period” model with an explicit active-process wait.
- If the interactive flow is redirected to the clipboard instead of creating a file, the result is treated as unsupported in v1 and no figure is attached.
- Cancellation or missing output file is treated as a no-op, not an error that aborts dictation.
- Failure to capture because Screen Recording permission is denied is also treated as a no-op for the session, but the user receives localized guidance for fixing permissions.

### 3. Storage model
- During an active session, screenshots are stored under an app-managed session directory:
  - `<app-data>/multimodal/sessions/<session-id>/`
- When the transcription is committed to history, successful session figures are promoted into stable history storage:
  - `<app-data>/history/figures/<entry-id>/`
- History stores a nullable serialized sidecar:
  - `ActiveContext { app_name, window_title, url, selected_text }`
  - `CapturedFigure { index, absolute_path, captured_at, anchor_sentence_ordinal }`
  - `MultimodalSidecar { active_context, figures, payload_version }`
- `transcription_text` remains spoken transcript text.
- `post_processed_text` remains refined spoken transcript text.
- `inserted_text` remains the exact pasted payload.

If screenshot promotion partially fails, the transcription still succeeds. Only successfully promoted figures remain in the sidecar and rendered payload.

### 4. Alignment and reinsertion
- Each captured figure records `captured_at` during the active session.
- After transcription completes, figures are aligned to sentence boundaries using transcription timing when available.
- The stored durable anchor is `anchor_sentence_ordinal`, not raw segment offsets.
- AI post-processing and `refine_last_transcript` receive spoken text only.
- After AI returns, the final spoken text is split into sentence boundaries and figures are reinserted in original order using the stored sentence ordinal.
- If a stored ordinal no longer maps cleanly to the refined text, remaining figures are appended at the end of the text body before the footer.

### 5. History and replay semantics
- History row previews use spoken/refined text only and may add multimodal badges/counts.
- Exact replay uses `inserted_text` when available.
- If a multimodal entry lacks `inserted_text`, the app may deterministically re-render from spoken text plus `MultimodalSidecar`.
- `paste_last_transcript` replays the original multimodal payload exactly; it does not adaptively rewrite the structured payload.

### 6. Permissions and assumptions
- Context capture may require Accessibility and per-app Automation permissions.
- Screenshot capture may require macOS Screen Recording permission.
- AppleScript-based context capture is acceptable because the current app is not App Sandbox enabled today. If the product adopts App Sandbox later, this design must be revisited because cross-app automation and file access rules tighten substantially.

## Risks / Trade-offs
- Owning screenshot capture intentionally narrows v1 compared with full macOS screenshot interoperability.
  - Mitigation: preserve Apple’s interactive region, window, and multi-display UX via `screencapture -i`.
- Exact replay and compact History now depend on sidecar persistence.
  - Mitigation: treat the sidecar as first-class history data instead of inferring it from rendered text.
- Reinserting figures after AI refinement can drift when text changes heavily.
  - Mitigation: use durable sentence ordinals and end-of-body fallback rather than fragile raw offsets.
- Interactive capture can remain open while the user moves across monitors or hesitates before selecting a region.
  - Mitigation: anchor figure timing to capture-action acceptance and allow only one in-flight screenshot process per session.

## Migration Plan
- Add a nullable history field for serialized multimodal sidecar metadata.
- Existing rows remain valid with `multimodal_sidecar = NULL`.
- Existing voice-only behavior remains unchanged when the multimodal setting is disabled or no sidecar exists.

## Open Questions
- None for v1. The proposal intentionally excludes passive system screenshot observation and push-to-talk screenshot support to keep implementation deterministic.
