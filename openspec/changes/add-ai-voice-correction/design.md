## Context
The goal is to provide seamless, context-aware voice correction. The key challenge is reliably capturing text context from arbitrary external applications on macOS.

## Goals / Non-Goals
- **Goals**: 
    - Enable "IDE-like" correction (Ghost Text) in any text input field.
    - Low latency (<500ms preferred) for the correction suggestion to feel immediate.
    - Strict privacy: Context data never leaves the device (Local LLM only).
- **Non-Goals**:
    - Replacing the operating system's comprehensive spell checker.
    - Supporting Linux/Windows in the initial rollout (macOS focus first).

## Decisions
- **Decision 1**: Use macOS Accessibility API (`AXUIElement`) as the primary method for text context.
    - **Why**: It allows reading text without interfering with the clipboard or focus, providing a better UX than "Copy/Paste" hacks.
    - **Trade-off**: It requires explicit permissions and can be flaky with non-native apps (Electron).
- **Decision 2**: Use "Ghost Text" overlay UI.
    - **Why**: Least intrusive interaction model. Users are familiar with it from coding assistants (Copilot).
    - **Trade-off**: Requires precise window positioning.
- **Decision 3**: Local LLM (MLX/Qwen) for intelligence.
    - **Why**: Privacy and offline capability.
    - **Risk**: Latency on older M1/M2 chips. Mitigated by using smaller quantized models (Qwen 2.5/3 4B/0.5B).
- **Decision 4**: Prompt Interpolation Strategy.
    - **Why**: To support correction, the prompt architecture must support dynamic context variables beyond just `${output}`.
    - **Variables**:
        - `${context}`: The surrounding text (e.g., 50 words before/after).
        - `${selection}`: The text currently selected (or word under cursor).
        - `${cursor_position}`: Indicator of where the cursor is relative to context.

## Edge Cases
- **Cursor at end of input**:
    - AI treats as "continuation/completion" or "last word correction".
    - UI appends ghost text.
- **Cursor in middle of word**:
    - System expands selection to the nearest word boundary to identify the target word.
    - UI prioritizes showing the correction **adjacent to the right** of the incorrect word (vs overlaying it), to avoid visual clutter and ensure readability.
- **Selection active**:
    - System treats the selection as the "target" for correction.
    - AI prompted to "Fix this specific text".
- **No Text Context (Empty Field)**:
    - **Decision**: Show the Correction Overlay with a "No text to correct" message.
    - **Why**: Keeps interaction model consistent (at cursor). Less intrusive than a system notification.
- **Permission Denied**:
    - System shows a specialized "Permissions Required" dialog once, then falls back to Clipboard (if enabled) or fails.
- **AI Returns No Change**:
    - UI shows "No changes detected" briefly, then fades out.

## Risks / Trade-offs
- **Risk**: `AXUIElement` fails to get text or cursor position.
    - **Mitigation**: Implement a robust fallback that simulates user input (Cmd+A -> Cmd+C) to capture context via clipboard, though this disrupts selection. Use sparingly/as user opt-in fallback.
- **Risk**: AI hallucination (changing meaning).
    - **Mitigation**: Use a very strict system prompt (`fix-homophones`) that penalizes changing words unnecessarily.

## Migration Plan
- N/A (New feature)
