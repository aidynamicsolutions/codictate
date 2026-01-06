# Tasks: Add MLX Local AI Provider

## 1. Backend Infrastructure

- [ ] 1.1 Add `mlx-rs = "0.21.0"` dependency to `Cargo.toml` with macOS aarch64 conditional compilation
- [ ] 1.2 Create `src-tauri/src/managers/mlx.rs` with `MlxModelManager` struct
- [ ] 1.3 Implement model registry with available models (Qwen 3, Gemma 3, SmolLM 3, Ministral 3) with Qwen 3 Base 1.7B as default
- [ ] 1.4 Implement model download from Hugging Face Hub with progress events
- [ ] 1.5 Implement download cancellation support
- [ ] 1.6 Implement download retry logic (max 3 attempts)
- [ ] 1.7 Implement model loading/unloading with state management
- [ ] 1.8 Implement model switching (unload current, reset state for new)
- [ ] 1.9 Implement text generation API (adapt from mlx-rs mistral example)
- [ ] 1.10 Add MLX manager initialization in `lib.rs` (macOS aarch64 only)
- [ ] 1.11 Integrate with existing model unload timeout setting
- [ ] 1.12 Implement error handling (network errors, disk space, corrupted files)
- [ ] 1.13 Implement empty directory cleanup on status check

## 2. Tauri Commands

- [ ] 2.1 Create `src-tauri/src/commands/mlx.rs` with command handlers
- [ ] 2.2 Implement `mlx_list_models` command
- [ ] 2.3 Implement `mlx_get_model_status` command
- [ ] 2.4 Implement `mlx_download_model` async command with progress events
- [ ] 2.5 Implement `mlx_cancel_download` command
- [ ] 2.6 Implement `mlx_retry_download` command
- [ ] 2.7 Implement `mlx_delete_model` command (with busy check)
- [ ] 2.8 Implement `mlx_process_text` async command
- [ ] 2.9 Register MLX commands in Tauri builder (macOS aarch64 only)
- [ ] 2.10 Add specta TypeScript bindings generation

## 3. Settings Integration

- [ ] 3.1 Add `LOCAL_MLX_PROVIDER_ID` constant to `settings.rs`
- [ ] 3.2 Add MLX provider to `PostProcessProvider` list
- [ ] 3.3 Add `selected_mlx_model` field to settings
- [ ] 3.4 Update `maybe_post_process_transcription()` to handle local-mlx provider

## 4. Frontend - Settings UI

- [ ] 4.1 Update post-processing provider dropdown to include "Local (MLX)" option
- [ ] 4.2 Create `MlxModelSelector` component with model list and status
- [ ] 4.3 Add download progress indicator with cancel button
- [ ] 4.4 Add retry button for failed downloads
- [ ] 4.5 Add model delete confirmation dialog
- [ ] 4.6 Show storage usage and model size information
- [ ] 4.7 Handle and display error states (network, disk, loading failures)
- [ ] 4.8 Conditionally render MLX options (hide on non-Apple Silicon)

## 5. TypeScript Bindings & Types

- [ ] 5.1 Add `MlxModelInfo` and `MlxModelStatus` types to bindings
- [ ] 5.2 Add event type for `mlx-model-state-changed`
- [ ] 5.3 Create `useMlxModels` hook for model state management

## 6. Testing & Verification

- [ ] 6.1 Test model download with progress tracking
- [ ] 6.2 Test download cancellation mid-download
- [ ] 6.3 Test download retry after failure
- [ ] 6.4 Test model loading and text generation
- [ ] 6.5 Test post-processing pipeline with local provider
- [ ] 6.6 Test graceful degradation on non-Apple Silicon platforms
- [ ] 6.7 Test model deletion and cleanup
- [ ] 6.8 Test prevent delete while model is busy (downloading/loading/running)
- [ ] 6.9 Verify TypeScript bindings are generated correctly
- [ ] 6.10 Test model unload timeout behavior
- [ ] 6.11 Test model switching behavior (unload old, load new)
- [ ] 6.12 Test fallback to original transcription when model not downloaded

## 7. Documentation

- [ ] 7.1 Update CLAUDE.md with MLX development notes
- [ ] 7.2 Add user-facing documentation for local AI feature
- [ ] 7.3 Document supported models and hardware requirements

## Dependencies

- Task group 2 depends on 1 (commands need manager)
- Task group 3 depends on 1, 2 (settings integration needs backend)
- Task group 4 depends on 2, 3, 5 (UI needs commands and types)
- Task group 6 depends on all above

## Parallelizable Work

- 1.2-1.6 can be developed incrementally
- 4.1-4.6 can be developed in parallel with backend once types are defined
- 5.x can be done alongside 2.x

## Estimated Effort

- Backend (tasks 1-3): ~3-4 days
- Frontend (tasks 4-5): ~2-3 days
- Testing & docs (tasks 6-7): ~1-2 days
- **Total: ~6-9 days**
