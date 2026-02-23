# Git Workflow for This Fork

This repository is a custom fork of [Handy](https://github.com/cjpais/Handy). We use a two-track branch model:

- `upstream-main`: clean mirror of `upstream/main` (no custom commits)
- `main`: Codictate custom work (features, fixes, integrations)

All new product work should be done through short-lived feature branches that start from `main`.

## Branch Roles

| Branch | Purpose | Rules |
| --- | --- | --- |
| `upstream-main` | Mirror of open source upstream | Keep clean. Only fast-forward from `upstream/main`. |
| `main` | Primary integration branch for Codictate | Merge feature branches and periodic upstream updates. |
| `feature/*`, `fix/*`, `chore/*` | Short-lived development branches | Branch from `main`, merge back to `main`, then delete. |
| `codex/safety-*` | Recovery points before risky merges | Create before upstream sync or large branch merges. |

## One-Time Setup (New Clone)

```bash
git remote -v
git remote add upstream https://github.com/cjpais/Handy.git  # if missing

git fetch upstream
git checkout -B upstream-main upstream/main
git branch --set-upstream-to=upstream/main upstream-main

git checkout main
git branch --set-upstream-to=origin/main main
```

## Feature Branch Workflow (Daily)

1. Sync local branches:

```bash
git fetch origin --prune
git checkout main
git pull --ff-only origin main
```

2. Create feature branch from `main`:

```bash
git checkout -b feature/short-description
```

3. Implement changes and commit with conventional commits (`feat:`, `fix:`, `docs:`, etc).
4. Push branch and open PR into `main`.
5. After merge, delete the feature branch.

## Periodic Upstream Sync Workflow

Run this weekly (or when upstream ships changes you want).

1. Fetch latest remotes:

```bash
git fetch upstream --prune
git fetch origin --prune
```

2. Update clean mirror branch:

```bash
git checkout upstream-main
git merge --ff-only upstream/main
git push origin upstream-main
```

3. Create a safety branch before touching `main`:

```bash
git checkout main
git pull --ff-only origin main
DATE=$(date +%F)
git branch "codex/safety-main-pre-upstream-merge-$DATE" main
git push origin "codex/safety-main-pre-upstream-merge-$DATE"
```

4. Merge upstream changes into custom `main`:

```bash
git merge upstream-main
```

5. Resolve conflicts, run verification, then push:

```bash
bun run lint
bun run test
# Optional when Rust/backend changed:
cargo test

git push origin main
```

## Merge History Documentation Requirement

Every upstream sync merge into `main` must be documented in:

- `doc/mergehistory/`

Use:

- `doc/mergehistory/README.md` for conventions
- `doc/mergehistory/TEMPLATE.md` for the file template

Current archive includes older merges from the previous flow where upstream updates were merged into `llm` and later `main`.

## Recovery Notes

If a sync merge causes problems:

1. Compare with safety branch:
   - `git log --oneline --graph --decorate main codex/safety-main-pre-upstream-merge-YYYY-MM-DD`
2. Roll forward with a fix commit when possible.
3. If rollback is necessary, reset local `main` to safety branch and force-push only with team approval.
