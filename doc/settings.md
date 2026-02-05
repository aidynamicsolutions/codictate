# Settings

This document outlines the general configuration options available in Codictate.

## General

### Keyboard Shortcuts
Configure the global proper hotkeys to control dictation.

-   **Push to Talk**: Hold the configured key combination to record. Recording stops when you release the keys.
    -   **Default (macOS)**: `fn` (Function key)
    -   **Default (Windows/Linux)**: `Ctrl` + `Shift`

### Microphone
Select the input device used for capturing speech.

-   **Input Device**: Choose from available system microphones.
-   **Visual Feedback**: A visual bar indicates input levels when speaking to help verify the microphone is working.

### Language
Configure the language used for speech-to-text transcription.

-   **Language Selection**: Choose a specific language (e.g., "English") for best accuracy.
-   **Auto-detect**: Automatically detects the spoken language.
    -   *Note*: Auto-detect may be slightly less accurate than selecting a specific language.
-   **Favorites**: You can select up to 8 languages to keep in your quick-access list.
-   **Active Language**: Click a language in your list to make it the primary active language.

## Sound

### Audio Feedback
Toggle sound effects for user interactions.

-   **Audio Feedback**: Enable/disable sounds when recording starts and stops.
-   **Output Device**: Select where feedback sounds are played (only enabled if Audio Feedback is on).
-   **Volume**: Adjust the playback volume for feedback sounds.

## Developer Notes

### Changing Default Values
When changing a default value for an existing setting in `src-tauri/src/settings.rs`, simply updating the default function (e.g., `default_some_setting()`) is **not sufficient** for existing users.

The configuration file (`settings.json`) persists the previous value. If the key exists in the user's file, the new default in code will be ignored.

**Reset All Settings:**
You can reset all settings to their default values in the **General > Advanced** section.
1. Scroll down to the "Advanced" group.
2. Click "Reset All Settings".
3. Confirm the action in the dialog.
*Note: This will not delete your recordings, history, or custom words.*

This forces `serde` to treat it as a missing key for existing users, effectively applying the new default value.
