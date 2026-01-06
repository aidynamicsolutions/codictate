# macOS Folder Analysis: WritingTools Application

## Yes, This is the macOS Application

The `macOS` folder at `/Users/tiger/Dev/opensource/speechGen/WritingTools/macOS` contains the **complete macOS application** for the WritingTools project. It includes:

- **WritingTools.xcodeproj** — The Xcode project file
- **WritingTools/** — The main Swift source code directory (84 items)

---

## MLX Framework Usage for Local AI

The application uses Apple's **MLX framework** to run AI models fully on-device on Apple Silicon Macs. Here's how it leverages MLX for text correction and generation based on text selection:

### Core MLX Implementation Files

| File | Purpose |
|------|---------|
| [LocalModelProvider.swift](WritingTools/Models/AI%20Providers/LocalModelProvider.swift) | Main MLX integration—downloads, loads, and runs local LLMs |
| [LocalModelInfo.swift](WritingTools/Models/AI%20Providers/LocalModelInfo.swift) | Defines available local models (LLM and VLM types) |
| [ModelConfiguration.swift](WritingTools/Models/AI%20Providers/ModelConfiguration.swift) | Additional MLX model configurations |

### MLX Imports Used

```swift
import MLX
import MLXVLM          // Vision Language Models
import MLXLLM          // Large Language Models
import MLXLMCommon     // Common utilities
import MLXRandom       // Random number generation for inference
```

---

## How Text Selection is Processed with MLX

### 1. Text Selection Capture (Automatic—No Manual Copy Required!)

> [!IMPORTANT]
> **Users do NOT need to manually copy text.** Simply **highlight/select text** in any app and press the hotkey—the app automatically captures the selection.

When the user triggers the app (via hotkey), here's what happens behind the scenes:

#### Step-by-Step Process

1. **Save current clipboard** — The app snapshots the existing clipboard contents
2. **Simulate Cmd+C** — The app programmatically sends a copy command to capture the highlighted text
3. **Read the copied text** — The newly copied selection is read from the clipboard
4. **Restore original clipboard** — The original clipboard contents are restored (user's clipboard is untouched!)

#### Implementation in AppDelegate.swift

```swift
// 1. Snapshot the clipboard BEFORE copying
let clipboardSnapshot = pb.createSnapshot()

// 2. Simulate Cmd+C to copy the user's selection
let src = CGEventSource(stateID: .hidSystemState)
let kd = CGEvent(keyboardEventSource: src, virtualKey: 0x08, keyDown: true)  // 'C' key
let ku = CGEvent(keyboardEventSource: src, virtualKey: 0x08, keyDown: false)
kd?.flags = .maskCommand
ku?.flags = .maskCommand
kd?.post(tap: .cgSessionEventTap)
ku?.post(tap: .cgSessionEventTap)

// 3. Wait for clipboard to update, then read the selection
await waitForPasteboardChange(pb, initialChangeCount: oldChangeCount)
let selectedText = pb.string(forType: .string) ?? ""

// 4. Restore the original clipboard (transparent to the user)
pb.restore(snapshot: clipboardSnapshot)
```

#### Key Benefits

| Feature | Description |
|---------|-------------|
| **Zero friction** | Just highlight and press hotkey—no Cmd+C needed |
| **Clipboard preserved** | Original clipboard contents are restored after capture |
| **Works anywhere** | Captures text from any app that supports system copy |
| **Rich text support** | Also captures attributed text (formatting) when available |

### 2. AI Provider Protocol

All providers (including MLX local) implement the `AIProvider` protocol:

```swift
protocol AIProvider {
    var isProcessing: Bool { get set }
    func processText(systemPrompt: String?, userPrompt: String, 
                     images: [Data], streaming: Bool) async throws -> String
    func cancel()
}
```

### 3. MLX Text Processing

The `LocalModelProvider.processText()` method (line 611) handles text generation:

```swift
func processText(systemPrompt: String?, userPrompt: String, 
                 images: [Data], streaming: Bool = false) async throws -> String
```

It determines whether to use:
- **VLM (Vision Language Model)** — for processing images + text
- **LLM (Language Model)** — for text-only processing

### 4. Generation with MLX

Actual inference happens via `MLXLMCommon.generate()`:

```swift
let stream = try MLXLMCommon.generate(
    input: input,
    parameters: parameters,
    context: context
)
```

---

## Available Local MLX Models

| Model | Type | Display Name |
|-------|------|--------------|
| `llama3_2_3B_4bit` | LLM | Llama 3.2 (3B, 4-bit) |
| `qwen3_4b_4bit` | LLM | Qwen 3.0 (4B, 4-bit) |
| `gemma3n_E4B_it_lm_4bit` | LLM | Gemma 3n IT (4B, 4-bit) |
| `gemma-3-4b-it-qat-4bit` | VLM | Gemma 3 VL (4B, 4-bit) 📷 **(Recommended)** |
| `qwen3vl_3b_instruct_4bit` | VLM | Qwen 3 VL (4B, 4-bit) 📷 |
| `qwen2_5vl_3b_instruct_4bit` | VLM | Qwen 2.5 VL (3B, 4-bit) 📷 |

---

## Writing Options That Use MLX

The selected text is processed based on `WritingOption` commands:

| Option | Task |
|--------|------|
| **Proofread** | Correct grammar, spelling, and punctuation |
| **Rewrite** | Rephrase while maintaining meaning |
| **Friendly** | Make text warmer and more approachable |
| **Professional** | Make text more formal and business-appropriate |
| **Concise** | Condense text while preserving essential information |
| **Summary** | Create a structured summary of key points |
| **Key Points** | Extract and list main points |
| **Table** | Organize information in a Markdown table |

---

## Summary

The `macOS` folder is the complete macOS application that uses the **MLX Swift framework** (`mlx-swift-examples`) to run quantized LLMs and VLMs **fully on-device** on Apple Silicon. The flow is:

1. **User selects text** → captured via system clipboard
2. **User triggers action** → via hotkey or popup menu
3. **Text sent to MLX model** → `LocalModelProvider.processText()`
4. **MLX generates response** → using `MLXLMCommon.generate()`
5. **Result replaces selection** → or displayed in response window
