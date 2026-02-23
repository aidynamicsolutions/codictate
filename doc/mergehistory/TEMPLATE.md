# Merge upstream-main -> main (YYYY-MM-DD)

## Context

- Source: `upstream-main` at `<SOURCE_SHA>`
- Target: `main` at `<TARGET_SHA_BEFORE>`
- Result: `<TARGET_SHA_AFTER>`

## Commits Reviewed from Upstream

| SHA | Description | Decision |
| --- | --- | --- |
| `<sha>` | `<summary>` | `Take` / `Skip` / `Partial` + reason |

## Conflict Resolution

| File | Strategy | Notes |
| --- | --- | --- |
| `<path>` | Keep ours / take upstream / hybrid | `<why>` |

## Verification

| Command | Result |
| --- | --- |
| `bun run lint` | PASS / FAIL |
| `bun run test` | PASS / FAIL |
| `cargo test` (if needed) | PASS / FAIL / N/A |

## Final Status

- Merge commit: `<MERGE_COMMIT_SHA>`
- Pushed branches:
  - `main`
  - `upstream-main`
  - `codex/safety-main-pre-upstream-merge-<date>`
- Follow-up tasks: `<none>` or list
