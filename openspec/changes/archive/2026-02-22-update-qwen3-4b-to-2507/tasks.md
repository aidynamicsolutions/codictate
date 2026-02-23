## 1. Registry Update
- [x] 1.1 Update `qwen3_base_4b.hf_repo` to `mlx-community/Qwen3-4B-Instruct-2507-4bit`.
- [x] 1.2 Keep `qwen3_base_4b` model ID unchanged.
- [x] 1.3 Update only `qwen3_base_4b.size_bytes` to the latest 2507 MLX size used for app preflight/display.
- [x] 1.4 Update only `qwen3_base_4b.parameters` text to latest minimum/typical RAM guidance.

## 2. Spec and Documentation
- [x] 2.1 Add a `local-ai-inference` spec delta for in-place 4B upgrade semantics.
- [x] 2.2 Add direct HF auth behavior note for public ungated vs gated/private repositories.
- [x] 2.3 Add documentation note that official Qwen `2507` equivalents are currently unavailable for 0.6B/1.7B/8B.

## 3. Validation
- [x] 3.1 Validate model registry behavior with tests (4B repo mapping and unchanged adjacent Qwen repos).
- [x] 3.2 Confirm disk preflight still uses updated 4B `size_bytes` field.
- [x] 3.3 Run `openspec validate update-qwen3-4b-to-2507 --strict`.
