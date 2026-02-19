# Modes Feature — Future Vision

> This document captures the long-term vision for evolving Codictate's "Refine" feature into a full **Modes** system, inspired by SuperWhisper.

---

## Phase 1: Refine (Current Plan)

| Item | Status |
|------|--------|
| Rename "Post Processing" → "Refine" | ⬜ |
| Always show Refine in sidebar | ⬜ |
| Off by default, toggle on in Refine page | ⬜ |
| Add `refine_last_transcript` hotkey | ⬜ |

---

## Phase 2: Modes Vision

### What Is a Mode?

A **Mode** bundles all settings for a specific workflow into one switchable preset:

```
Mode = {
  name: "Professional Email",
  icon: "✉️",
  sttModel: "parakeet-multilingual",  // Which transcription model
  llmProvider: "local-mlx",           // Which AI provider
  llmModel: "qwen-0.5b",              // Which LLM model
  language: "en",                     // Source language
  prompt: "Format as professional...", // Custom instructions
  hotkey: "cmd+shift+e"               // Optional dedicated hotkey
}
```

### Built-in Presets

| Mode | STT Model | LLM | Prompt Summary |
|------|-----------|-----|----------------|
| **Raw** | Fast | None | No AI, just transcription |
| **Refine** | Best | Local | Clean grammar, format naturally |
| **Professional** | Best | Local | Formal tone, proper structure |
| **Casual** | Fast | Local | Friendly, conversational |
| **Notes** | Best | Local | Bullet points, headers |
| **Code** | Best | Local | Variable names, syntax-aware |

### UI Changes Required

1. **New "Modes" sidebar section** (replaces Refine)
2. **Mode list view** with create/edit/delete
3. **Quick mode picker** in home page or overlay
4. **Per-mode settings panel**:
   - Name & icon
   - STT model dropdown
   - LLM provider + model
   - Language
   - Prompt textarea
   - Hotkey binding
5. **Mode indicator** in recording overlay

### Technical Considerations

- **Data structure**: New `Mode` type in settings
- **Migration**: Convert existing prompts → modes
- **Hotkey registration**: Dynamic per-mode hotkeys
- **Default mode**: Which mode triggers on main hotkey

---

## Competitor Reference: SuperWhisper

SuperWhisper's Modes feature includes:
- **Super mode**: Context-aware, adapts to active app
- **Voice to text**: Raw, minimal processing
- **Message/Mail/Note/Meeting**: Task-specific presets
- **Custom**: User-defined with all settings exposed

Key differentiator: One-click mode switching from sidebar.

---

## Questions to Resolve

1. Should modes have dedicated hotkeys, or use a mode picker + main hotkey?
2. How to handle mode switching mid-session?
3. Should there be a "Super" context-aware mode like SuperWhisper?
4. Per-mode vocabulary/dictionary support?
