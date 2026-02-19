## Context
The application currently hardcodes `https://blob.handy.computer` for model downloads. The user wants to host models on their own CDN (Cloudflare) for production release.

## Goals / Non-Goals
- **Goal**: Allow changing the CDN base URL without modifying code logic in multiple places.
- **Goal**: specific support for the user's Cloudflare setup.
- **Non-Goal**: Dynamic runtime configuration of CDN URL by the end-user (this is a build-time/deployment configuration).

## Decisions
- **Decision**: Use a `const` or `static` for the base URL, potentially overridable at build time via `env!`.
- **Rationale**: Minimal complexity. The URL is an infrastructure constant, not a user preference.

## Risks / Trade-offs
- **Risk**: User-hosted CDN must have the same directory structure as the current blob storage.
- **Mitigation**: Document the expected directory structure clearly.

## Migration Plan
1. Update code to use the constant.
2. Verify against the existing `blob.handy.computer` (it should still work as the default).
3. User changes the constant/env var for their production build.
