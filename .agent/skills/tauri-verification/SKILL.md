---
name: tauri-verification
description: Verifies the functionality of the Tauri application in a browser environment using robust mocks. Use this when the user wants to check if the app is working correctly or when debugging rendering issues (white screens) in the browser.
---

# Tauri Verification

This skill helps verifying the Handy Tauri application in a standard browser environment (like Chrome or Headless Chrome via `browser_subagent`) by injecting robust mocks for the native Tauri APIs.

## When to Use

- Verifying the application loads and renders correctly.
- Debugging "white screen" issues where the app fails to mount.
- Checking specific flows (Onboarding, Dashboard) without needing a full Tauri build.
- Resolving `TypeError: Cannot read properties of undefined` related to `window.__TAURI__`.

## Workflow

### 1. Setup Mocks

Run the included script to generate the `src/mocks/tauri.ts` file and inject the import into `src/main.tsx`.

```bash
python3 .agent/skills/tauri-verification/scripts/setup_mocks.py
```

This script ensures:
- `src/mocks/tauri.ts` exists with robust mocks for `invoke`, `event`, and `plugins`.
- `import "./mocks/tauri";` is the **first** import in `src/main.tsx`.

### 2. Ensure Development Server is Running
    
Check if the development server is already active:

```bash
python3 .agent/skills/tauri-verification/scripts/check_server.py
```

- If it returns **"Server is running."** (exit code 0), proceed to the next step.
- If it returns **"Server is NOT running."** (exit code 1), start it in the background:

```bash
bun tauri dev
```
*(Wait 10-15 seconds for the server to be ready before proceeding).*

### 3. Verify in Browser

Use the `browser_subagent` tool to verify the application.

**Example Verification Task:**

```text
Navigate to http://localhost:1420.
Wait 5 seconds.
Check if the Dashboard is visible (Look for 'History', 'Settings').
If Onboarding appears, verify navigation through steps.
Take a screenshot of the final state.
```

### 4. Cleanup (Required)

**CRITICAL:** You **MUST** remove the mock import to restore the native application's functionality. Leaving the mocks active will cause the native app to fail (e.g., showing a **black screen**) because it will try to use browser mocks instead of real Tauri APIs.

```bash
# Clean up mocks and restore src/main.tsx
python3 .agent/skills/tauri-verification/scripts/setup_mocks.py --cleanup
```

## Troubleshooting

- **White Screen / Crash on Load**: Usually means mocks are not loaded *before* dependencies or a command is missing.
    - **Missing Mocks**: If the console shows `TypeError: Cannot read properties of null` or similar, it often means a Tauri command returned `null` instead of an expected object. Check the console for failed commands and update `setup_mocks.py` to include them.
    - **Load Order**: Ensure `import "./mocks/tauri";` is the **first** import in `src/main.tsx`.
- **Data Mismatches**: If stats or history entries don't appear, check `src/bindings.ts` to ensure your mock objects in `setup_mocks.py` match the exact TypeScript interface (e.g., `transcription_text` vs `transcription`).
- **`unregisterListener` Error**: Indicates the `window.__TAURI_INTERNALS__.event` mock is missing or incomplete. The `setup_mocks.py` script includes a fix for this.
- **Null Stats Error**: If `Home.tsx` crashes, check if `get_home_stats` mock is returning valid data (handled in `setup_mocks.py`).
