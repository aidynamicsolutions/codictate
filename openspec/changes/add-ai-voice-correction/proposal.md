# Change: Add AI Voice Correction

## Why
Users currently struggle with occasional ASR errors (homophones, small mistakes) that break their flow. Fixing these requires switching context to the keyboard. An "IDE-like" correction flow using local AI can fix these errors instantly using context, keeping the user in the creative flow.

## What Changes
- **New Capability**: Context-Aware Correction triggered by a global shortcut.
- **Backend**:
    - Integration with macOS Accessibility API (`AXUIElement`) to read text context from any active application.
    - Fallback mechanism using Clipboard (Cmd+C/V) for incompatible apps.
    - "Correction Pipeline" orchestrated by a new `CorrectionManager`.
    - **Prompt Engine Upgrade**: Support for new interpolation variables `${context}` and `${selection}`.
- **Frontend**:
    - "Ghost Text" overlay UI that renders suggested changes near the cursor.
- **AI**:
    - Specialized prompt for `python-backend` (Qwen/MLX) to perform strict text correction based on context.

## Impact
- **Affected Specs**: `accessibility`, `ui`, `ai`
- **Affected Code**: `src-tauri/src/accessibility/`, `src-tauri/src/overlay.rs`, `src/overlay/CorrectionView.tsx`, `python-backend/server.py`
- **New Permissions**: Users must grant Accessibility permissions to the app.
