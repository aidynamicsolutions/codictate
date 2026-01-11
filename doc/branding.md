# Branding Guide

This guide explains how to rebrand the Handy application, including changing colors, the app name, and the logo/icons.

## Quick Reference

| Element | File(s) to Edit |
|---------|-----------------|
| Colors | `src/App.css`, `src/theme.ts` |
| App Name | `src/i18n/locales/*/translation.json` (change `appName` key) |
| App Name (Tauri) | `src-tauri/tauri.conf.json` |
| Logo/Icons | Run `bun tauri icon` with new source image |

---

## Color Theming

All brand colors are centralized in two files:

### 1. CSS Variables (`src/App.css`)

The primary source of truth for runtime theming:

```css
@theme {
  --color-logo-primary: #faa2ca;    /* Main pink - edit this! */
  --color-background-ui: #da5893;   /* UI accent */
  --color-logo-stroke: #382731;     /* Logo outline */
  --color-bar: #ffe5ee;             /* Recording bars */
  --color-border: #ff69b4;          /* Recording border */
  /* ... other colors */
}
```

Dark mode overrides are in the `@media (prefers-color-scheme: dark)` section.

### 2. TypeScript Constants (`src/theme.ts`)

For type-safe access in JavaScript/TypeScript code:

```typescript
import { colors } from "@/theme";

// Use in inline styles
style={{ backgroundColor: colors.primary }}
```

### Changing the Primary Color

1. Edit `src/App.css`:
   - Change `--color-logo-primary` to your new brand color
   - Update dark mode variant in the `@media` block
2. Edit `src/theme.ts`:
   - Update `colors.primary` and related values
3. Rebuild the app: `bun run tauri dev`

---

## App Name

The app name "Handy" is centralized in translation files.

### Frontend Translations

Each language file has an `appName` key:

```json
{
  "appName": "Handy",
  ...
}
```

**Files to edit:**
- `src/i18n/locales/en/translation.json`
- `src/i18n/locales/es/translation.json`
- `src/i18n/locales/fr/translation.json`
- `src/i18n/locales/de/translation.json`
- `src/i18n/locales/zh/translation.json`
- `src/i18n/locales/vi/translation.json`
- `src/i18n/locales/pl/translation.json`

### Backend Translations

The Rust backend also has translation files:

**Files to edit:**
- `src-tauri/resources/locales/en/translation.json`
- `src-tauri/resources/locales/*/translation.json` (all other languages)

### Tauri Configuration

Edit `src-tauri/tauri.conf.json`:

```json
{
  "productName": "Handy",  // Change this
  "identifier": "com.pais.handy",  // Update identifier
  "app": {
    "windows": [
      {
        "title": "Handy",  // Change this
        ...
      }
    ]
  }
}
```

### Cargo Configuration

Edit `src-tauri/Cargo.toml`:

```toml
[package]
name = "handy"           # Change to new app name (lowercase)
description = "Handy"    # Change to new app name
default-run = "handy"    # Change to new app name (lowercase)

[lib]
name = "handy_app_lib"   # Change to newname_app_lib
```

### Rust Backend Files

These files contain hardcoded app name references:

| File | What to Update |
|------|----------------|
| `src-tauri/src/main.rs` | `handy_app_lib::run()` → `newname_app_lib::run()` |
| `src-tauri/src/llm_client.rs` | HTTP headers (User-Agent, X-Title, Referer) |
| `src-tauri/src/tray.rs` | Version label format string, tray icon path |
| `src-tauri/src/tracing_config.rs` | Log file prefix, doc comment |
| `src-tauri/src/managers/history.rs` | WAV file name prefix |

### Tray Icon (Linux)

If rebranding, rename the colored tray icon:

```bash
# Rename the icon file
mv src-tauri/resources/handy.png src-tauri/resources/newname.png

# Update reference in src-tauri/src/tray.rs:
# Line ~56: "resources/handy.png" → "resources/newname.png"
```

---

## Logo and Icons

Tauri has a built-in command to generate all required icon sizes from a single source image.

### Requirements

- **Source image**: 1024x1024 pixels (minimum)
- **Format**: PNG with transparency
- **Location**: Project root, named `app-icon.png`

### Generate Icons

```bash
# Place your new logo as app-icon.png in project root
bun tauri icon

# Or specify a custom path
bun tauri icon /path/to/your/logo.png
```

This generates all required icons in `src-tauri/icons/`:
- `icon.icns` (macOS)
- `icon.ico` (Windows)
- `icon.png` and various sizes (Linux/general)
- iOS and Android icons (in subdirectories)

### Generated Files

The command creates icons for all platforms:

| Platform | Files |
|----------|-------|
| macOS | `icon.icns` |
| Windows | `icon.ico`, `Square*.png` files |
| Linux | `32x32.png`, `128x128.png`, `128x128@2x.png` |
| iOS | `ios/AppIcon-*.png` |
| Android | `android/` directory |

### SVG Logo

The text logo (used in the UI) is an inline SVG component:

**File:** `src/components/icons/HandyTextLogo.tsx`

This component uses CSS classes `logo-primary` and `logo-stroke` which inherit from CSS variables, so changing colors in `App.css` will update this logo automatically.

---

## Complete Rebranding Checklist

- [ ] **Colors**
  - [ ] Update `src/App.css` (light mode colors)
  - [ ] Update `src/App.css` (dark mode colors)
  - [ ] Update `src/theme.ts`
  
- [ ] **App Name**
  - [ ] Update all frontend translation files (`appName` key)
  - [ ] Update all backend translation files
  - [ ] Update `src-tauri/tauri.conf.json` (`productName`, `title`, `identifier`)
  - [ ] Update `src-tauri/Cargo.toml` (`name`, `description`, `default-run`, lib `name`)
  
- [ ] **Rust Backend**
  - [ ] Update `src-tauri/src/main.rs` (lib crate reference)
  - [ ] Update `src-tauri/src/llm_client.rs` (HTTP headers)
  - [ ] Update `src-tauri/src/tray.rs` (version label, icon path)
  - [ ] Update `src-tauri/src/tracing_config.rs` (log file prefix)
  - [ ] Update `src-tauri/src/managers/history.rs` (WAV file prefix)
  - [ ] Rename `src-tauri/resources/handy.png` to new name

- [ ] **Logo/Icons**
  - [ ] Create 1024x1024 PNG source image
  - [ ] Run `bun tauri icon`
  - [ ] (Optional) Update `HandyTextLogo.tsx` SVG paths

- [ ] **Verify**
  - [ ] Run `cargo check` in `src-tauri/` for Rust errors
  - [ ] Run `bun run build` to check frontend
  - [ ] Test in dev mode: `bun run tauri dev`
  - [ ] Check light and dark mode
  - [ ] Verify app name appears correctly everywhere

---

## Troubleshooting

### Colors not updating?

1. Clear browser cache / restart dev server
2. Check for hardcoded values in component files
3. Verify CSS variable syntax is correct

### Icons not showing?

1. Ensure source image is at least 1024x1024
2. Check `src-tauri/icons/` for generated files
3. Rebuild: `bun run tauri build`

### App name not changing everywhere?

Search the codebase for hardcoded instances:
```bash
grep -r "Handy" src/ src-tauri/
```
