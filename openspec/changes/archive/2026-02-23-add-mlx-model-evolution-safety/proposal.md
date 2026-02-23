# Change: MLX Model Evolution Safety (CDN-Ready, HF-Only Now)

## Why
The app will eventually have production users. MLX model selection is persisted by model ID, and model files are stored under ID-keyed directories. If IDs are renamed or models are replaced without migration/fallback rules, users can hit broken selections or stale local model state.

We need a production-safe model evolution strategy now, while keeping delivery practical before CDN infrastructure is ready.

## What Changes
- Introduce immutable, versioned canonical model IDs for MLX catalog entries.
- Add alias resolution and persisted selection migration for backward compatibility.
- Add deterministic fallback when a selected model is missing or invalid.
- Define source-aware catalog metadata and downloader dispatch interface now.
- Ship with Hugging Face source only for now; reserve mirror/CDN source for later.
- Keep remote signed catalog and mirror failover out of implementation scope for this change, but prepare schema and interfaces for it.

### Future Hosting Flexibility (CDN-Ready by Design)
- The system models model-download sources as an ordered list per model.
- Initial deployment uses Hugging Face source only.
- Mirror/CDN source can be added later without changing selection migration or catalog schema.
- Remote manifest support is out-of-scope for this implementation, but schema/interface seams are prepared.

## Impact
- Affected specs: `local-ai-inference`
- Affected code (implementation target):
  - `src-tauri/src/managers/mlx/manager.rs`
  - `src-tauri/src/managers/mlx/catalog.rs` (new)
  - `src-tauri/src/managers/mlx/downloader.rs` (new)
  - `src-tauri/src/managers/mlx/provider.rs` (new)
- Compatibility:
  - Existing external Tauri MLX commands remain unchanged.
  - Existing `update-qwen3-4b-to-2507` change remains untouched.
