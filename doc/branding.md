# Codictate Branding Guidelines

This document serves as the source of truth for the Codictate application branding, including logos, icons, and typography.

## Logo Architecture

The Codictate logo is a distinct "Wink Face" emblem (`^ _ *`). It replaces the previous "Hand" logo.

### 1. In-App Logo
Used in the application Sidebar and UI.

*   **Component**: `src/components/icons/CodictateLogo.tsx`
*   **Format**: SVG Component.
*   **Style**: transparent background, stroke-based.
*   **Color**: Inherits text color (`currentColor`) via Tailwind classes (`fill-text`, `stroke-text`).
*   **Border**: 4px stroke width.
*   **Typography**: "Codictate" text appears below the logo in the sidebar (`font-bold text-lg tracking-tight`).

### 2. App Icons (Desktop & Mobile)
Used for the application binary, mobile home screens, and dock/taskbar.

*   **Source File**: `src-tauri/resources/svg/app_logo.svg`
*   **Location**: `src-tauri/icons/` (generated files).
*   **Style**:
    *   **Background**: Solid White (`#FFFFFF`).
    *   **Stroke/Content**: Dark Gray (`#333333`).
    *   **Stroke Width**: 8px (Thick border style).
*   **Generation**: Use `bun run tauri icon src-tauri/resources/svg/app_logo.svg`.

### 3. macOS Tray Icons
Used in the macOS menu bar. These are specialized monochrome icons to handle light/dark mode visibility.

*   **Location**: `src-tauri/resources/`
*   **Files**:
    *   `tray_idle.png`: **White** stroke (`#FFFFFF`), transparent background. Used when the system menu bar is Dark.
    *   `tray_idle_dark.png`: **Black** stroke (`#000000`), transparent background. Used when the system menu bar is Light.
*   **Style**: 8px stroke width (matches App Icon thickness).
*   **Note**: These are effectively template icons but handled explicitly by the backend to ensure high visibility contrast.

## Asset Generation Workflow

### Regenerating App Icons
If the main logo design changes in `app_logo.svg`:
1.  Ensure `app_logo.svg` has the "App Icon" styling (Solid White background, Dark content).
2.  Run:
    ```bash
    bun run tauri icon src-tauri/resources/svg/app_logo.svg
    ```

### Regenerating Tray Icons
Tray icons require specific coloring (Pure White / Pure Black) and transparency.
1.  Modify `app_logo.svg` to be Pure White (stroke only, transparent background).
2.  Run `tauri icon` or manually export to `src-tauri/resources/tray_idle.png` (resize to 44x44px).
3.  Modify `app_logo.svg` to be Pure Black.
4.  Export to `src-tauri/resources/tray_idle_dark.png`.
5.  **Important**: Revert `app_logo.svg` to the standard "App Icon" style (White bg / Dark content) after generating tray icons.

## File Locations
*   **Source SVG**: `src-tauri/resources/svg/app_logo.svg`
*   **Generated Icons**: `src-tauri/icons/`
*   **Tray Resources**: `src-tauri/resources/tray_*`
*   **React Component**: `src/components/icons/CodictateLogo.tsx`
