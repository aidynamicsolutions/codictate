# Fork + Upstream + Sentry Secrets Checklist

Use this checklist when the original GitHub repo is not under your control and you still need CI secrets (for Sentry sourcemap uploads).

Goal:

1. Keep pulling updates from the original open-source repository.
2. Push your own branches to your fork.
3. Store Sentry GitHub secrets safely in your fork.

## Quick Mental Model

1. `upstream` = original repo (read/sync source of truth).
2. `origin` = your fork (write/push, CI, secrets).

## Daily/Weekly Sync Quick Commands

Use this after initial setup to keep your fork and `llm` branch current:

```bash
git fetch upstream
git checkout main
git merge upstream/main
git push origin main

git checkout llm
git merge main
git push origin llm
```

Quick checks:

- [ ] `git remote -v` shows `origin` = your fork and `upstream` = original repo.
- [ ] `main` has latest upstream commits.
- [ ] `llm` has latest `main` merged.

Important rule:

- [ ] Always sync `main` from `upstream/main` before pushing `main` to your fork.

## Phase 1: Create and Confirm Fork

- [ ] In GitHub UI, fork `cjpais/Handy` to your own account.
- [ ] Confirm your fork URL exists: `https://github.com/<your-username>/Handy`.

## Phase 2: Rewire Local Git Remotes

Run in local repo root:

```bash
git remote -v
git remote rename origin upstream
git remote add origin https://github.com/<your-username>/Handy.git
git remote -v
```

Expected result:

1. `origin` points to your fork.
2. `upstream` points to `https://github.com/cjpais/Handy.git`.

Checklist:

- [ ] `origin` is your fork.
- [ ] `upstream` is original repo.

## Phase 3: Push Base Branches to Your Fork (Sync First)

```bash
git fetch upstream
git checkout main
git merge upstream/main
git push -u origin main

git checkout llm
git merge main
git push -u origin llm
```

Checklist:

- [ ] local `main` is synced from `upstream/main`.
- [ ] `main` exists on your fork.
- [ ] `llm` exists on your fork.

## Phase 4: Add Sentry Secrets to Your Fork

In your fork GitHub UI:

1. `Settings -> Secrets and variables -> Actions`
2. Click `New repository secret` for each value.

Required secrets:

1. `SENTRY_AUTH_TOKEN`
2. `SENTRY_ORG`
3. `SENTRY_PROJECT`

How to get values:

1. `SENTRY_AUTH_TOKEN`: from Sentry org auth tokens, scopes:
   - `project:releases`
   - `org:read`
2. `SENTRY_ORG`: your Sentry org slug.
3. `SENTRY_PROJECT`: your Sentry project slug (for example: `codictate`).

Checklist:

- [ ] All three secrets are created in your fork.

## Phase 5: Keep Syncing from Original Repo

Use this routine whenever you want latest upstream changes:

```bash
git fetch upstream
git checkout main
git merge upstream/main
git push origin main

git checkout llm
git merge main
git push origin llm
```

Checklist:

- [ ] Local `main` includes `upstream/main`.
- [ ] Fork `main` is updated.
- [ ] `llm` includes latest `main`.

## Phase 6: Validate Sentry CI Path on Fork

1. Push a commit to fork branch.
2. Run GitHub Actions build in your fork.
3. Confirm build does not fail from missing Sentry vars.
4. Confirm sourcemap upload step runs when secrets are present.

Checklist:

- [ ] Fork workflow runs successfully.
- [ ] Sentry sourcemaps upload in CI.

## Troubleshooting

### I still push to original repo by accident

Run:

```bash
git remote -v
```

If `origin` is not your fork, repeat Phase 2.

### I get permission denied when pushing

You are likely pushing to `upstream` by mistake. Push to `origin`:

```bash
git push origin <branch-name>
```

### CI says Sentry secret is missing

Secrets must be configured in your fork repository settings, not upstream.

### No symbolicated frontend stack traces

Check:

1. `SENTRY_AUTH_TOKEN`, `SENTRY_ORG`, `SENTRY_PROJECT` exist in fork secrets.
2. runtime `release` and CI upload `release` match (`codictate@<version>`).
