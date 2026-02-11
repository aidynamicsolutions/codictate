## 1. Accessibility & Context Capture
- [ ] 1.1 Create `src-tauri/src/accessibility/mod.rs` and implementation for macOS `AXUIElement` interaction.
- [ ] 1.2 Implement `get_focused_element`, `get_selected_text_range`, and `get_text_context`.
- [ ] 1.3 Implement fallback strategies (Clipboard copy/paste simulation).
- [ ] 1.4 Add new permissions handling logic for Accessibility access.
- [ ] 1.5 Implement "Smart Selection" logic: Expand cursor position to word boundaries if no text selected.

## 2. Correction Manager
- [ ] 2.1 Create `src-tauri/src/managers/correction.rs` to orchestrate the flow.
- [ ] 2.2 Wire up the Trigger (Global Shortcut).
- [ ] 2.3 Implement the connection to `python-backend` AI inference.
- [ ] 2.4 Implement Prompt Interpolation: Support `${context}` and `${selection}` variables.
- [ ] 2.5 Handle text replacement logic (via Accessibility API or simulated keystrokes).

## 3. UI Implementation
- [ ] 3.1 Update `overlay.rs` to support `Correction` state and window positioning near cursor.
- [ ] 3.2 Create `src/overlay/CorrectionView.tsx` with "Ghost Text" styling (faded/grey text).
- [ ] 3.3 Add visual indicators for "Tab to accept" and "Esc to dismiss".
- [ ] 3.4 Implement "Adjacent Positioning" logic: Ensure correction appears to the right of the target word, not on top of it.
- [ ] 3.5 Handle "No Text" state: Show a transient "No text to correct" message in the overlay.

## 4. Integration & Testing
- [ ] 4.1 Verify context capture across different apps (Notes, VS Code, Browser).
- [ ] 4.2 Validate AI correction accuracy with the new prompt and variable substitution.
- [ ] 4.3 Test edge cases (no context, permission denied, slow AI response, cursor in middle of word).
