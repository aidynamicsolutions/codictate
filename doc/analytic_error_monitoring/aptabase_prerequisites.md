# Aptabase Analytics Prerequisites

To enable Aptabase analytics for Handy, you need to provide the App Key.

## 1. Create an Aptabase Project
1.  Log in to [Aptabase.com](https://aptabase.com/).
2.  Create a new project.
3.  Name the project `handy-app` (or similar).
4.  Select **Tauri** as the framework if asked (or just Generic).

## 2. Get the App Key
1.  Go to **Project Settings**.
2.  Copy the **App Key** (it usually looks like `A-US-1234567890`).

## 3. Configuration
You have two options to provide this key:

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
