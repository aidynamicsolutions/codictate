## Track A — Implement Now (HF-Only Runtime)

## 1. Catalog and ID Semantics
- [x] 1.1 Introduce canonical versioned IDs for MLX model entries.
- [x] 1.2 Add alias metadata and canonical resolution logic.

## 2. Selection Safety
- [x] 2.1 Normalize selected model ID (alias -> canonical) before MLX operations.
- [x] 2.2 Persist normalized canonical ID back to settings.
- [x] 2.3 Implement deterministic fallback for missing/invalid selected IDs.

## 3. Source-Aware Schema (HF enabled only)
- [x] 3.1 Add source-aware catalog models (`ModelSource`, catalog structs).
- [x] 3.2 Add source-dispatch downloader interface.
- [x] 3.3 Keep catalog sources HF-only for shipped behavior.
- [x] 3.4 Ensure behavior remains unchanged for actual downloads.

## 4. Validation
- [x] 4.1 Add tests: alias resolution + persisted migration.
- [x] 4.2 Add tests: unknown-ID fallback behavior.
- [x] 4.3 Add tests: source schema parsing and HF-only dispatch.
- [x] 4.4 Run `openspec validate add-mlx-model-evolution-safety --strict`.

## Track B — Deferred (Requires CDN Readiness)

> Deferred until mirror/CDN infrastructure is available.

- [ ] B.1 Implement `http_mirror` downloader path.
- [ ] B.2 Implement source-priority failover (mirror -> HF) at runtime.
- [ ] B.3 Implement remote catalog provider fetch + signature verification.
- [ ] B.4 Add rollout controls and operational documentation for catalog hosting.
