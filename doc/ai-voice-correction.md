# AI Voice Correction

Corrects ASR (Automatic Speech Recognition) errors — homophones, misheard words, and similar-sounding substitutions — using a local LLM.

## Quick Start

1. Type or dictate text in any app
2. Select the word(s) to correct
3. Press **Fn+Z**
4. Review the ghost-text overlay showing the correction
5. Press **Tab** to accept or **Esc** to dismiss

## How It Works

```
User selects text → Fn+Z → Capture context via Accessibility API
  → Build prompt (context + selection + examples)
  → Send to LLM (local MLX or Apple Intelligence)
  → Extract correction for selected region
  → Show overlay with ghost text
  → Tab (accept & replace) / Esc (dismiss)
```

### Context-Aware Correction

The correction pipeline sends the **full surrounding context** to the LLM, not just the selected word. This is critical for homophones like `their/they're/there` where the correct word depends entirely on the sentence.

- **Selected text**: What the user highlighted (can be one word or an entire paragraph)
- **Context**: The surrounding text captured via the macOS Accessibility API
- **`${output}`**: Receives the full context sentence so the LLM sees the word in its natural position
- **`${context}`**: Also available for additional surrounding text reference

After the LLM returns the corrected sentence, the pipeline extracts only the correction for the selected region using word-offset alignment.

## Architecture

### Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/managers/correction.rs` | Core correction pipeline, prompt building, result extraction |
| `src-tauri/src/actions.rs` | `CorrectAction` — orchestrates the correction flow |
| `src-tauri/src/fn_key_monitor.rs` | Handles Fn+Z shortcut (Fn key requires native CGEventTap) |
| `src-tauri/src/accessibility/mod.rs` | Context capture via macOS Accessibility API |
| `src/overlay/RecordingOverlay.tsx` | Overlay UI (shows correction ghost text) |
| `prompts/correct-text-v8.md` | Hardcoded correction prompt (embedded via `include_str!`) |

### Pipeline Flow

1. **`fn_key_monitor.rs`** detects `Fn+Z`, cancels any active PTT recording, dispatches `correct_text` action
2. **`CorrectAction::start()`** checks guards (not recording, not already correcting), shows processing overlay
3. **`CorrectionManager::run_correction()`** runs the async pipeline:
   - Captures context via Accessibility API (`selected_text` + `context`)
   - Determines whether to send full context or just selection to LLM
   - Builds prompt using hardcoded v8 template + `interpolate_prompt()`
   - **Scans selection** for known homophones (tokens) and phonetic slips (phrases) to inject specific hints
   - Injects strict dictionary replacements (capped at 50 items)
   - Sends to configured LLM provider (local MLX, Apple Intelligence, or remote API)
   - Calls `extract_selected_correction()` to map corrected words back to selected region
4. **Result handling**:
   - `has_changes=true` → Show correction overlay with ghost text
   - `has_changes=false` → Auto-dismiss after 1.5s (no changes needed)

### Extraction Logic (`extract_selected_correction`)

When the full context is sent to the LLM, we need to map the corrected output back to the user's selection:

1. Find the byte offset of the selected text in the original context
2. Count words before the selection (prefix) and words in the selection
3. Extract the corresponding word range from the corrected output
4. Fallback: if word counts don't align, try suffix matching

Example:
```
Context:  "their going to the park"
Selected: "their"
LLM returns: "They're going to the park"
Extraction: prefix_words=0, selected_words=1 → "They're"
```

## Prompt Engineering

The correction prompt is **hardcoded** in `correction.rs` via `include_str!("prompts/correct-text-v8.md")`. It is **not** user-configurable — the Fn+Z shortcut always uses this tested prompt, fully decoupled from the refine prompt system.

The v8 prompt uses a **Strict JSON** structure optimized for reliability and speed:

```
System: Correction Engine. Return JSON ONLY.

Task: Correct "selection" using "context".
- Apply [Slip] aggressively.
- Apply [Hint] ONLY if selection doesn't fit context.
- Output thought & correction.
- If correct, return original.

Examples:
Input: {"context": "I think their coming", "selection": "their"}
Hints: - [Hint] 'their' might be meant as 'they're'
Output: {"thought": "Context 'coming' -> 'they are'.", "correction": "they're"}
...
```

### Template Variables

| Variable | Value |
|----------|-------|
| `${output}` | Full context sentence (or selected text if context unavailable) |
| `${context}` | Surrounding text for additional reference |
| `${selection}` | The user's original selection |
| `${hints}` | List of injected hints (Slips, Homophones, Replacements) |

### Advanced Strategy: Counter-Intuitive Examples

We deliberately include "technically correct but contextually wrong" examples to break the model's bias towards high-frequency words.

- **Example**: `Input: "I know one is left"` -> `Output: "no one"`
- **Why**: "I know one is left" is a valid sentence. However, in rapid dictation, "know one" is almost always a misinterpretation of "no one".
- **Effect**: By forcing this correction, we teach the model to prioritize **contextual probability** (quantity "one" implies negation "no") over **literal validity**. Without this, the model fails to correct "know one" 100% of the time.

### Dictionary Injection & Scanned Hints

The prompt acts on specific hints derived from the user's dictionary and internal resources:

- **Phonetic Slips** (`phonetic_slips.json`): scanned phrase matches (e.g. "ten to" -> "tend to").
- **Visual Hints** (`homophones.json`): scanned token matches (e.g. "allowed" -> "might be meant as 'aloud'").
- **Replacement entries** (Settings): strict replacements defined by the user.

**Note**: Global vocabulary injection (`- Vocabulary: 'Kubernetes'`) has been **removed** to reduce prompt noise and prevent hallucinations. The model relies on its internal knowledge for general vocabulary unless a specific replacement is defined.

### Model Considerations

- **Qwen 2.5 3B** (local MLX): Optimized sampling — temperature=0.0 (greedy), top_p=1.0. Prioritizes deterministic, safe corrections.
- **Larger models** (8B+): Better at catching all homophones in complex sentences.
- **Apple Intelligence**: Alternative provider, uses system-level model.

## Configuration

- **Shortcut**: Configurable in Settings → Keyboard Shortcuts (default: `Fn+Z`)
- **LLM Provider**: Settings → Post-Processing → Provider
- **Dictionary**: Settings → Dictionary (entries are injected as hints into the correction prompt)
- **Model**: Settings → Post-Processing → Model selection

## Limitations

- **Single-pass correction**: The LLM processes text once. Very dense errors (7+ per sentence) may not all be caught by smaller models.
- **Homophone-only**: Designed for acoustic errors, not grammar or style. The prompt explicitly instructs "DO NOT fix grammar or style."
- **Context window**: The surrounding context captured by the Accessibility API is limited to what's visible/accessible in the active app.
- **macOS only**: Context capture relies on the macOS Accessibility API. Other platforms use stub implementations.

## Debugging

Run with `bun run tauri:dev:debug` and look for these log lines:

```
# What was captured
Captured text for correction selected="their" context="their going to the park"

# What was sent to the LLM
Text being sent to LLM use_full_context=true text_for_llm="their going to the park"

# Raw LLM response (after Python-side clean_model_response)
Sidecar /generate response raw_response="They're going to the park"

# Extraction result
Extracted correction for selected region prefix_words=0 selected_words=1 corrected_selected="They're"

# Final result
Correction pipeline complete has_changes=true
```

The Python sidecar (`python-backend/server.py`) also logs the full formatted prompt and raw model output in the terminal.
