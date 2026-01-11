# Production Release Checklist

This document lists all items that must be completed before releasing to production on macOS.

---

## App Configuration

### HTTP Client Headers (`src-tauri/src/llm_client.rs`)

Update the placeholder URLs to your production values:

```rust
// Line ~41: Update REFERER to your actual domain
HeaderValue::from_static("https://codictate.app"),  // ← Change to real domain

// Line ~45: Update USER_AGENT with version info
HeaderValue::from_static("Codictate/1.0"),  // ← Update version as needed
```

### Tauri Configuration (`src-tauri/tauri.conf.json`)

1. **Bundle Identifier**: Update for production
   ```json
   "identifier": "com.yourcompany.codictate"
   ```

2. **Auto-Updater Endpoint**: Add your update server URL
   ```json
   "plugins": {
     "updater": {
       "endpoints": [
         "https://your-update-server.com/latest.json"
       ]
     }
   }
   ```

3. **macOS Code Signing**: Configure in `bundle.macOS`
   ```json
   "macOS": {
     "signingIdentity": "Developer ID Application: Your Name (XXXXXXXXXX)",
     "hardenedRuntime": true,
     "entitlements": "Entitlements.plist"
   }
   ```

---

## App Store / Notarization

### macOS Notarization

Before distributing outside the App Store:

1. **Apple Developer Account**: Ensure you have a valid Developer ID certificate
2. **Notarization**: Run notarization after building:
   ```bash
   xcrun notarytool submit path/to/app.dmg --apple-id YOUR_APPLE_ID --password APP_SPECIFIC_PASSWORD --team-id TEAM_ID
   ```
3. **Stapling**: After notarization succeeds:
   ```bash
   xcrun stapler staple path/to/app.dmg
   ```

---

## Version Management

### Before Each Release

1. **Update version** in these files:
   - `src-tauri/tauri.conf.json` → `"version": "x.y.z"`
   - `src-tauri/Cargo.toml` → `version = "x.y.z"`
   - `package.json` → `"version": "x.y.z"`

2. **Generate changelog** for the release

---

## Build Commands

### Development Build
```bash
bun run tauri dev
```

### Production Build (macOS)
```bash
bun run tauri build
```

Output location: `src-tauri/target/release/bundle/`

---

## Pre-Release Verification

- [ ] All version numbers match across config files
- [ ] HTTP client headers use production URLs
- [ ] Update server endpoint is configured (if using auto-updates)
- [ ] Code signing identity is set correctly
- [ ] App icon and branding are correct
- [ ] Test on clean macOS installation
- [ ] Verify notarization succeeds
- [ ] Test auto-update flow (if applicable)

---

## Files Reference

| Purpose | File |
|---------|------|
| Version | `tauri.conf.json`, `Cargo.toml`, `package.json` |
| HTTP Headers | `src-tauri/src/llm_client.rs` |
| Bundle ID | `tauri.conf.json` → `identifier` |
| macOS Signing | `tauri.conf.json` → `bundle.macOS` |
| Auto-Updater | `tauri.conf.json` → `plugins.updater` |
