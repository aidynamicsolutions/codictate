# Change: Add MLX-Based Local AI Provider for Transcription Enhancement

## Why

Currently, Handy relies on **cloud-based LLM APIs** (OpenAI, Anthropic, OpenRouter, etc.) to post-process transcriptions on macOS. This requires API keys, internet connectivity, and incurs usage costs. The macOS WritingTools app demonstrates a successful pattern using Apple's **MLX framework** to run quantized LLMs fully on-device on Apple Silicon Macs, providing:

- **Zero latency network calls** — All inference happens locally
- **Privacy** — User data never leaves the device
- **No API costs** — Uses free local models
- **Offline capability** — Works without internet

## Research Validation

| Finding | Source |
|---------|--------|
| mlx-rs is feature-complete (v0.21.0) | [mlx-rs GitHub](https://github.com/oxideai/mlx-rs) |
| MLX ~1.14x faster than llama.cpp on M3 | Reddit benchmarks Dec 2024 |
| Has working Mistral text generation example | mlx-rs/examples/mistral |
| Industry recommends direct Rust bindings for Tauri | Tauri LLM best practices |
| MLX optimized for Apple Silicon unified memory | Apple MLX documentation |

## What Changes

### New Capability: Local AI Inference Provider
A new local AI provider that uses **mlx-rs** (Rust bindings for MLX) to run quantized LLMs on Apple Silicon Macs for transcription enhancement.

### Detailed Changes

1. **New module: `src-tauri/src/managers/mlx.rs`**
   - Model management (download, load, unload, cache)
   - Text generation API matching the existing `AIProvider` pattern
   - Support for LLM models (Qwen 3, Gemma 3, SmolLM 3, Ministral 3)
   - Progress reporting for model downloads

2. **Settings integration**
   - Add "Local (MLX)" as a post-processing provider option
   - Model selection dropdown with available local models
   - Download/delete model actions
   - **Note**: Only available on macOS Apple Silicon (aarch64)

3. **Frontend updates**
   - Model download UI with progress indicator
   - Model status display (downloaded, downloading, loading, ready)
   - Storage usage information

4. **Integration with existing post-processing pipeline**
   - Update `maybe_post_process_transcription()` to support local provider
   - Seamless fallback to original transcription if local inference fails

### Models to Support Initially

**Default Model:** ⭐ **Qwen 3 Base 1.7B** — Best balance of speed and quality for most users

| Model ID | Display Name | Size | Notes |
|----------|--------------|------|-------|
| `qwen3_base_1.7b` | Qwen 3 Base 1.7B | 1.0 GB | ⭐ **Default** — Best balance |
| `qwen3_base_4b` | Qwen 3 Base 4B | 2.3 GB | Higher quality for powerful machines |
| `qwen3_base_0.6b` | Qwen 3 Base 0.6B | 0.4 GB | Fastest, smallest |
| `qwen3_base_8b` | Qwen 3 Base 8B | 4.7 GB | Best quality (16GB+ RAM) |
| `gemma3_base_4b` | Gemma 3 Base 4B | 2.3 GB | Multi-language support |
| `gemma3_base_1b` | Gemma 3 Base 1B | 0.8 GB | Small multi-language |
| `gemma3_base_270m` | Gemma 3 Base 270M | 0.3 GB | Smallest multi-language |
| `smollm3_base_3b` | SmolLM 3 Base 3B | 1.8 GB | Multi-language alternative |
| `ministral3_base_3b` | Ministral 3 Base 3B | 2.0 GB | Mistral family |


## Impact

- **Affected code:**
  - `src-tauri/src/actions.rs` — Add local provider handling
  - `src-tauri/src/settings.rs` — Add MLX provider settings
  - `src-tauri/src/managers/` — New MLX model manager
  - `src/components/settings/` — Post-processing provider UI
  - `Cargo.toml` — Add mlx-rs dependency

- **Platform constraints:**
  - **macOS Apple Silicon only** — MLX requires ARM64 Metal support
  - Windows/Linux will continue using cloud providers

- **Dependencies:**
  - [`mlx-rs`](https://github.com/oxideai/mlx-rs) v0.21.0 — Rust bindings for MLX
  - Model files stored in `~/Library/Application Support/handy/mlx-models/`

## Decisions (Resolved Open Questions)

| Question | Decision | Rationale |
|----------|----------|-----------|
| VLM Support | **No** — LLM only for Phase 1 | Reduces complexity, can add later if needed |
| Model Loading | **On-demand** | User triggers load before first inference, match existing pattern |
| Memory Management | **Same unload timeout as transcription engine** | Consistency with existing behavior, reuse settings infrastructure |
| Model Downloads | **Hugging Face Hub** | Standard model hosting, like the reference WritingTools implementation |
