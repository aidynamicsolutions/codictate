## Context
MLX model evolution currently relies on static in-code entries with persisted model IDs and ID-keyed local storage paths. This is risky for production updates unless model ID evolution is explicitly managed.

CDN/mirror infrastructure is planned but not available yet.

## Goals / Non-Goals
- Goals:
  - Safe model evolution across app versions.
  - Backward-compatible selection behavior for persisted model IDs.
  - Forward-compatible catalog/downloader seams for future mirror/CDN support.
  - Preserve current HF-only operational behavior until CDN is ready.
- Non-Goals:
  - Implementing remote manifest fetch/verification now.
  - Implementing mirror downloader now.
  - Changing frontend/backend MLX command contracts now.

## Decisions

### Decision: Canonical versioned IDs
Each model SHALL have an immutable canonical ID. New model revisions use new canonical IDs.

### Decision: Alias + persisted migration
Legacy IDs SHALL map to canonical IDs via alias mapping.
When a legacy or invalid ID is detected at runtime, selection SHALL be normalized and persisted.

### Decision: Source-Abstraction Now
Catalog entries define ordered sources:
- `sources: [ModelSource]`
- `ModelSource` fields:
  - `kind`: `huggingface_repo | http_mirror`
  - `value`: repo ID or URL template
  - `priority`: lower value means earlier attempt
  - `enabled`: source activation flag
  - `allow_fallback`: whether next source may be attempted on failure

### Decision: Downloader Interface
Define internal source-dispatch API now:
- `download_from_source(source, file, progress) -> Result<Path>`

Implement now:
- `huggingface_repo` path only.

Reserve/stub now:
- `http_mirror` path returns explicit `NotConfigured` / equivalent non-fatal source error.

### Decision: Catalog Provider Interface
Define provider abstraction now:
- `CatalogProvider::load_catalog() -> Catalog`

Implement now:
- Embedded/local provider only.

Reserve for later:
- Remote signed provider returning the same catalog structure.

### Decision: Reserved config surface
Declare optional config keys now (inactive by default):
- `MLX_MIRROR_BASE_URL`
- `MLX_CATALOG_URL`
- `MLX_CATALOG_PUBKEY`

Unset behavior SHALL remain current HF-only.

## Data Model Changes
Planned internal models:
- `MlxCatalog`
- `MlxCatalogModel`
- `ModelSource`
- `CatalogProvider`
- `ModelDownloader` source-dispatch boundary

## Rollout Plan
- Phase A (this change): selection safety + source-aware schema + HF-only runtime.
- Phase B (future, CDN ready): mirror downloader + remote signed catalog + source-priority failover.

## Risks / Trade-offs
- Risk: alias mapping drift.
  - Mitigation: invariant tests (unique aliases, canonical target existence).
- Risk: stale local model directories under legacy IDs.
  - Mitigation: deterministic alias/canonical directory handling and non-destructive conflict policy.
- Risk: extra complexity from future-ready seams.
  - Mitigation: keep current runtime path HF-only and defer mirror execution logic.
