# Aptabase Analytics Prerequisites

To enable Aptabase analytics for Codictate, you need to provide the App Key.

## 1. Create an Aptabase Project
1.  Log in to [Aptabase.com](https://aptabase.com/).
2.  Create a new project.
3.  Name the project `codictate-app` (or similar).
4.  Select **Tauri** as the framework if asked (or just Generic).

## 2. Get the App Key
1.  Go to **Project Settings**.
2.  Copy the **App Key** (it usually looks like `A-US-1234567890`).

## 3. Configuration
The app resolves the Aptabase key in this order:
1. Runtime environment variable `APTABASE_APP_KEY`
2. Build-time embedded `APTABASE_APP_KEY` (`option_env!`)
3. If neither is available, analytics is disabled gracefully

You have two common ways to provide this key:

### Option A: `.env` File (Recommended for Dev)
Create or update the `.env` file in the root directory:
```bash
APTABASE_APP_KEY=your_app_key_here
```

### Option B: Build-time Variable
Pass it during the build process:
```bash
APTABASE_APP_KEY=your_app_key_here cargo tauri build
```

### Option C: CI Variable (Required for Distributed Builds)
For installers shipped to end users, the machine running the installed app usually does not provide runtime env vars.

Set a CI secret named `APTABASE_APP_KEY` and inject it into the Tauri build environment so the key is embedded at compile time (same model as `SENTRY_DSN`).

In this repo, GitHub Actions build workflow reads:
- `secrets.APTABASE_APP_KEY` → `APTABASE_APP_KEY` (build environment)

Without this CI value, installed builds will run with analytics disabled unless a runtime env var is manually provided.

## 4. Disable Controls
- Runtime kill switch:
  ```bash
  HANDY_DISABLE_ANALYTICS=1
  ```
  This disables analytics initialization even if a key is present.
- User opt-out:
  - In Settings, disable **Share Anonymous Usage Analytics**.
  - Event sending stops while the toggle is off.

## 5. Delivery Expectations
- Analytics delivery is **best-effort**.
- Offline events are retried by the Aptabase plugin queue.
- On graceful app exit, Codictate emits `app_exited`; Aptabase plugin flushes queued events during exit handling.
- Crash/force-kill scenarios are not guaranteed delivery paths.

## 6. Source of Truth by Stage
1. Local dev: `.env` / shell `APTABASE_APP_KEY`.
2. Distributed builds: CI-provided `APTABASE_APP_KEY` embedded at compile time.
3. Runtime override: shell `APTABASE_APP_KEY` can override embedded key for local debugging/reroute.
