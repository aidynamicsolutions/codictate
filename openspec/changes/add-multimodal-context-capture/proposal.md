# Change: Add Multimodal Context Capture

## Why
Users currently have no reliable way to attach application state or screenshots to a dictation session. The original proposal depended on passively observing macOS screenshot behavior, which leaves too many gaps around timing, file formats, permissions, and replay semantics. This change narrows v1 to a deterministic, app-owned multimodal flow that keeps the user experience strong while making the payload stable enough to implement and preserve.

The active-text capture fallback sequence continues to adapt proven ideas from the reference `aside` codebase in `/Users/tiger/Dev/opensource/macapp/aside`, but screenshot capture is intentionally redesigned to be app-owned rather than observed externally.

## What Changes
- Add a stable multimodal payload contract composed of:
  - an optional `Context:` block for app/window/URL/selection metadata,
  - dictated or refined spoken text with inline `[Figure N]` markers,
  - footer lines `Figure N: <absolute path>` for attached screenshots.
- Replace passive `CGEventTap` + screenshot-folder watching with an app-owned screenshot flow using `/usr/sbin/screencapture -i -o -x`, triggered by a dedicated configurable shortcut during active hands-free dictation only.
- Persist screenshot metadata as a `MultimodalSidecar` attached to history so:
  - spoken transcript text remains the source text for AI post-processing and refine,
  - pasted multimodal payloads can be replayed exactly,
  - History stays compact instead of showing file paths in the main preview.
- Add a dedicated screenshot shortcut binding `capture_dictation_screenshot` with macOS default `control+command+s`.
- Define explicit cleanup and retention rules for screenshot files promoted into app-managed history storage.
- Add a `design.md` that locks the rendering contract, sidecar data model, AI strip/reattach behavior, and the non-sandbox AppleScript assumption used for app/browser context capture.

## Impact
- Affected specs: `multimodal-context` (new), `transcript-insertion` (modified), `transcription-history` (modified), `local-ai-inference` (modified)
- Affected code: multimodal session capture state, screenshot action/shortcut handling, history persistence and cleanup, paste/render pipeline, settings/help text, and focused multimodal tests
