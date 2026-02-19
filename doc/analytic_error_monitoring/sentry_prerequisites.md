# Sentry Integration Prerequisites

To enable Sentry error monitoring for Codictate, you need to provide the following credentials.

## 1. Create a Sentry Project
1.  Log in to [Sentry.io](https://sentry.io/).
2.  Create a new project.
3.  Choose **Browser (JavaScript)** or **Rust** (the generic project type works for both in this context, but Browser is often easier for the frontend config).
4.  Name the project `codictate-app` (or similar).

## 2. Get the DSN (Data Source Name)
1.  Go to **Project Settings** > **Client Keys (DSN)**.
2.  Copy the DSN URL (e.g., `https://examplePublicKey@o0.ingest.sentry.io/0`).

## 3. Configuration
You have two options to provide this key:

### Option A: `.env` File (Recommended for Dev)
Create or update the `.env` file in the root directory:
```bash
SENTRY_DSN=your_dsn_here
```

### Option B: Build-time Variable
Pass it during the build process:
```bash
SENTRY_DSN=your_dsn_here cargo tauri build
```

## 4. Source Maps (Optional but Recommended)
For readable stack traces, you will need to upload source maps.
1.  Go to **Organization Settings** > **Auth Tokens**.
2.  Create a new token with `project:releases` scope.
3.  Add `SENTRY_AUTH_TOKEN` to your CI/CD secrets or local `.env`.
