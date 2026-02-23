# Change: Upgrade Qwen 3 Base 4B to Instruct 2507 (MLX)

## Why
The current MLX 4B entry points to an older Qwen 3 4B model. We need to upgrade to the latest 4B `2507` Instruct release while preserving existing user selection compatibility. This change also clarifies direct Hugging Face download behavior for public ungated MLX repositories.

## What Changes
- Update only `qwen3_base_4b` to use `mlx-community/Qwen3-4B-Instruct-2507-4bit`.
- Keep the existing model ID `qwen3_base_4b` unchanged so current settings continue to work without migration.
- Update only 4B model metadata:
  - `size_bytes` to match the `2507` MLX artifact sizing used by the app.
  - `parameters` display text to include minimum and typical RAM guidance.
- Keep `qwen3_base_0.6b`, `qwen3_base_1.7b`, and `qwen3_base_8b` entries unchanged.
- Document direct HF download expectations:
  - Public ungated models can be downloaded without user token.
  - Gated/private models require Hugging Face authentication.
- Document that official `2507` equivalents are not currently available for 0.6B/1.7B/8B.

## Impact
- Affected specs: `local-ai-inference`
- Affected code:
  - `src-tauri/src/managers/mlx/manager.rs`
  - `doc/local-ai-mlx.md`
- Backward compatibility:
  - No command/type changes
  - No settings migration required (ID stays `qwen3_base_4b`)
