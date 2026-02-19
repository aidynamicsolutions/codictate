# Change: Infrastructure Rebranding & Self-Hosted CDN

## Why
Currently, the application downloads models from `blob.handy.computer` and references `handy.computer` domains/repos throughout. To complete the rebranding to "Codictate" and ensure long-term reliability for the production release, we need to:
1. Switch to a self-hosted CDN (e.g., Cloudflare R2) under the user's control.
2. Update all project URLs, domains, and email addresses to the new `codictate.app` branding.

## What Changes
- **Model CDN**: Implement compile-time configuration for the model CDN URL to replace `blob.handy.computer`.
- **Project URLs**: Update all references to `github.com/cjpais/Handy` to `github.com/cjpais/codictate` (or configured repo).
- **Website/Email**: Update `handy.computer` to `codictate.app` and `contact@handy.computer` to `contact@codictate.app`.
- **Documentation**: Update `README.md`, `CONTRIBUTING.md`, and all docs to reflect these infrastructure changes.

## Impact
- **Affected Specs**: `model-management` (new capability), `project-identity` (implied)
- **Affected Code**: `src-tauri/src/managers/model.rs`, all documentation files.
- **Security**: Ensures model binary integrity and brand consistency.
