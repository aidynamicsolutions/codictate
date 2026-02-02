# AI Visual Verification for Tauri Apps

This document outlines the best practices and recommended workflow for enabling AI agents (specifically Antigravity) to visually verify the Handy Tauri application.

## Core Strategy: Web-Based Verification

Since Tauri interfaces are built with web technologies, the most robust and "agent-friendly" way to verify them is by treating the frontend as a standard web application served via the development server.

### Why not Native Window?
- **macOS Limitations**: Native WebDriver support for macOS desktop automation is fundamentally limited or non-existent in many standard toolchains.
- **Agent Capabilities**: AI agents excel at interacting with browsers (DOM access, JavaScript execution) compared to opaque native platform windows.

## Recommended Workflow

### 1. Verification Environment
Run the application in **headless/web mode** using the Vite dev server. This allows agents to access the UI at `http://localhost:1420` (or `http://localhost:5173`).

**Command:**
```bash
bun run dev
```

### 2. Mocking Native APIs
Because the browser environment lacks the `window.__TAURI__` context injection, calls to native APIs (clipboard, file system, window controls) will fail. 

**Solution:**
Create a mock layer for verification. You can detect the environment and provide mock responses.

```typescript
// Example: src/mocks/tauri.ts
if (typeof window !== 'undefined' && !window.__TAURI__) {
  console.log('Injecting Tauri Mocks for Browser Verification');
  
  // ... mock implementation ...
  // IMPORTANT: Mocks must match the Typescript interfaces in src/bindings.ts exactly.
  // Mismatched types (e.g. 'transcription' vs 'transcription_text') will cause data to not appear.
}
```

### 3. Cleanup: The "Black Screen" Danger
**CRITICAL:** Once verification is complete, you **MUST** remove the mock injection from `src/main.tsx`.

If mocks remain active when running the app natively (desktop mode), the app will attempt to use the browser mocks instead of real Tauri APIs, resulting in a **Black Screen** or reduced functionality.

**Always end a verification session with a cleanup step.**

### 4. Handling Specific Crashes (Case Study: Handy)

In this project, `src/i18n/index.ts` calls `commands.getAppSettings()` and `plugin-os` immediately on load. Without mocks, the app crashes with a white screen.

**Required Mocks for Handy:**
1.  **`get_app_settings`**: Must return a valid settings object or the i18n sync will fail.
2.  **`plugin-os`**: Must mock `locale()` to prevent top-level await failures.

To verify the app, you MUST inject these mocks before the app initializes. The standard `browser_subagent` can inject this via `execute_javascript` *before* the app loads if you use a preamble script, but it is reliable to include a `src/mocks.ts` imported only in `dev` mode.

### 4. Agent Verification Loop
The AI agent (Antigravity) can perform the verification loop as follows:

1.  **Start Server**: Agent runs `bun run dev` (or connects to existing instance).
2.  **Navigate**: Agent uses `browser_subagent` to visit `http://localhost:1420`.
3.  **Verify**:
    - **Visual**: Agent captures screenshots of specific routes/states.
    - **Functional**: Agent uses standard DOM interaction to click buttons, fill forms, and verify state changes.
4.  **Report**: Agent generates a `walkthrough.md` with screenshots and results.

## External Tools (CI/CD)

For automated pipelines outside of the interactive agent session, use **Playwright**:

- **Setup**: Playwright can connect to the Vite dev server or use `tauri-driver` (Linux/Windows).
- **Vision AI**: Tools like `Applitools` or `Percy` integrate with Playwright to provide pixel-perfect regression testing that agents can review.

## Summary
To enable the "AI verification loop":
1.  Ensure `bun run dev` works and serves the app.
2.  Implement basic mocks for critical Tauri commands if they block the UI load.
3.  Instruct the agent to "Open localhost:1420 and verify X" rather than "Open the app".
