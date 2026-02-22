# Sentry Integration Prerequisites

This file lists the minimum prerequisites before enabling Sentry in Codictate.

## 1. Sentry Access

1. Create or access a Sentry organization.
2. Create a project for Codictate desktop errors.
3. Confirm you can view project settings and project releases.

## 2. Required Values

### Runtime (local app)

- `SENTRY_DSN`: enables Sentry event ingestion and overrides embedded DSN when set.
- `SENTRY_ENVIRONMENT`: recommended values are `development`, `staging`, or `production`.
- `HANDY_DISABLE_SENTRY`: optional kill switch (`1` disables Sentry entirely).

### CI (sourcemap upload)

- `SENTRY_AUTH_TOKEN`
- `SENTRY_ORG`
- `SENTRY_PROJECT`
- `SENTRY_DSN` (required to embed DSN into distributed builds)
- `SENTRY_RELEASE` (must match runtime release tag)

## 3. Auth Token Scope (sourcemaps)

Create a Sentry auth token with:

- `project:releases`
- `org:read`

These scopes are sufficient for release creation and frontend sourcemap upload.

## 4. Release Naming Convention

Use a single convention everywhere:

- `codictate@<version>`

Examples:

- `codictate@0.7.6`
- `codictate@0.8.0-beta.1`

Runtime events and uploaded sourcemaps must use the same release string.

## 5. Delivery Model (Pre-Prod)

Current phase is pre-prod and uses best-effort delivery:

- no durable offline queue requirement
- no native minidump sidecar requirement

The app should continue running normally if Sentry is disabled, misconfigured, or unreachable.

## 6. DSN Source of Truth by Stage

1. Local dev: `.env` / shell `SENTRY_DSN`.
2. Distributed builds: CI-provided `SENTRY_DSN` embedded at compile time.
3. Runtime override: shell `SENTRY_DSN` can override embedded DSN for debugging/reroute.
