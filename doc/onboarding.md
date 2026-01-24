# Onboarding Flow Documentation

Comprehensive documentation of the Codictate onboarding experience.

## Flow Overview

```
Welcome → Attribution → Tell Us About You → Typing Use Cases → Permissions → Download Model → Microphone Check → Hotkey Setup → Language Select → Learn → Success → Referral
   1           2                3                  4               5             6                7                 8                9            10       11        12
```

Steps 1-4 collect user profile data. Steps 5-10 configure the app. Steps 11-12 introduce Pro features and referrals.

---

## Step Details

### 1. Welcome Step

**Component**: [WelcomeStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/WelcomeStep.tsx)

- **Input**: User's name (optional, 100 char limit)
- **Illustration**: `undraw_hey-by-basecamp_61xm.svg`
- **Persists to**: `user_name` in user profile

### 2. Attribution Step

**Component**: [AttributionStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/AttributionStep.tsx)

- **Question**: "Where did you hear about us?"
- **Selection type**: Single-choice
- **Options**: social_media, youtube, newsletter, ai_chat, search_engine, event, friend, colleague, podcast, article, product_hunt, other
- **Secondary options**: Social media platforms with icons (TikTok, Instagram, X, Discord, Facebook, LinkedIn, Reddit, Threads)
- **Illustration**: `undraw_welcome-cats_tw36.svg`
- **Persists to**: `referral_sources`, `referral_details` in user profile

### 3. Tell Us About You Step

**Component**: [TellUsAboutYouStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/TellUsAboutYouStep.tsx)

- **Question 1**: "What do you do for work?"
- **Options**: 17 work roles including developer, designer, manager, student, etc.
- **Question 2**: "Tell us more" (professional level)
- **Conditional**: Hidden for student, writer, customer_support, other
- **Levels**: executive, director, manager_lead, mid_level, entry_level, intern
- **Other input**: Text field when "Other" role selected
- **Illustrations**: 3 polaroid-style SVGs with tilts and hover animations
- **Persists to**: `work_role`, `work_role_other`, `professional_level`

### 4. Typing Use Cases Step

**Component**: [TypingUseCasesStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/TypingUseCasesStep.tsx)

- **Question**: "Where do you spend your time typing?"
- **Selection type**: Multi-select
- **Options**: ai_chat, messaging, coding, emails, documents, notes, social_posts, other
- **Other input**: Text field when "Something else" selected (100 char limit)
- **Illustrations**: Mood board layout with 4 SVGs in organic scatter
- **Persists to**: `typing_use_cases`, `typing_use_cases_other`

### 5. Permissions Step

**Component**: [PermissionsStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/PermissionsStep.tsx)

Requests macOS accessibility and microphone permissions.

**Features**:
- Two permission cards (Accessibility → Microphone)
- State machine per permission: `idle` → `checking` → `granted`
- Polling every 500ms to detect permission changes (60s timeout)
- Info icon with shadcn Tooltip explaining each permission
- Loader2 spinner during checking, green checkmark when granted
- Looping video tutorials on right panel (auto-switches per permission)
- Continue button enabled only when both permissions are granted

**Video files**:
- Accessibility: `/src-tauri/resources/videos/accessibilityPermission.webm`
- Microphone: `/src-tauri/resources/videos/micPermission11.57.35am_compressed.webm`

**Permission APIs** (from `tauri-plugin-macos-permissions-api`):
- `checkAccessibilityPermission()` / `requestAccessibilityPermission()`
- `checkMicrophonePermission()` / `requestMicrophonePermission()`

**System Settings URLs** (opened via `@tauri-apps/plugin-opener`):
- Accessibility: `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`
- Microphone: `x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone`

**Capabilities configuration** ([default.json](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/capabilities/default.json)):
```json
{
  "identifier": "opener:allow-open-url",
  "allow": [
    { "url": "https://*" },
    { "url": "http://*" },
    { "url": "mailto:*" },
    { "url": "x-apple.systempreferences:*" }
  ]
}
```

### 6. Download Model Step

**Component**: [ModelDownloadStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/ModelDownloadStep.tsx)

Downloads the recommended ASR model while allowing users to continue setup.

**Features**:
- Auto-selects best model via `getRecommendedFirstModel()` (Parakeet V3 for most systems)
- Model info card showing name, description, size, and speed score
- Download progress with percentage, speed, and estimated time
- Dynamic messaging encouraging users to continue during download
- Pulsating Continue button appears once download starts
- "Extracting..." state after download completes
- Note explaining model will be set as default (changeable in settings)

**Persistent Progress Indicator**: [ModelDownloadProgress.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/ModelDownloadProgress.tsx)
- Floating toast at bottom-right when user navigates away
- Collapsible with progress bar and speed display
- Shows "Ready to use!" with green checkmark for 5s after extraction completes

**Events**:
- `model-download-progress` - Progress updates during download
- `model-download-complete` - Download finished, extraction starts
- `model-extraction-started` - Extraction phase begins
- `model-extraction-completed` - Model ready to use
- `model-extraction-failed` - Error with retry option

**i18n keys**: `onboarding.downloadModel.*`, `modelSelector.extractingGeneric`

### 7. Microphone Check Step

**Component**: [MicrophoneCheckStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/MicrophoneCheckStep.tsx)

**Features**:
- Back button to return to Permissions step (fire-and-forget navigation)
- Left panel: Title "Speak to test your microphone" with subtitle
- Right panel: Card with live audio level visualization (16 bars)
- Question: "Do you see red bars moving while you speak?"
- "No, change microphone" button → opens selection Dialog
- "Yes" button → proceeds to HotkeySetup step

**Audio Level Visualization**:
- Uses AGC (Automatic Gain Control) for normalized display
- Tracks peak levels with attack/release dynamics (0.3s attack, 2s release)
- Ensures visible bar movement for any input level (like video conferencing apps)
- Smoothed level visualization to reduce jitter

**Dialog (microphone selection)**:
- Lists actual microphones (filters out "Default" meta-entry)
- Selected microphone appears at top of list
- System default mic shows "(default)" label
- Polls for new devices every 2 seconds while dialog is open
- Handles newly connected devices (e.g., AirPods) in real-time
- Level indicator on selected microphone

**Backend Commands**:
- `startMicPreview()` - Opens mic stream to emit levels without recording
- `stopMicPreview()` - Closes mic stream (fire-and-forget to prevent UI blocking)
- Listens to `mic-level` event from Rust backend

**Note on virtual devices**: Shows all audio devices including virtual ones (BlackHole, Microsoft Teams Audio) as power users may need them for audio routing.

### 8. Hotkey Setup Step

**Component**: [HotkeySetupStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/HotkeySetupStep.tsx)

> **See also**: [Hotkey Shortcut Documentation](file:///Users/tiger/Dev/opensource/speechGen/Handy/doc/hotkeyshortcut.md) for detailed technical documentation with ASCII diagrams.

**Features**:
- Displays Push to Talk (`fn`) and Hands-free mode (`fn+space`) shortcuts
- Each shortcut has inline recording via `ShortcutCard` component
- Uses shared hook: [useShortcutRecorder.ts](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/hooks/useShortcutRecorder.ts)
- Native macOS Fn key detection via Tauri events (`fn-key-down`, `fn-key-up`)
- ESC cancels recording
- Modifier key sorting for consistent display

**Mode Detection**:
- 150ms delay before push-to-talk starts (allows time to detect fn+space)
- If Space pressed within 150ms → hands-free mode
- If 150ms expires → push-to-talk mode
- Late fn+space (after PTT started) → cancels PTT and starts hands-free

**Shortcut Recording Hook**:
- Handles ESC to cancel, filters auto-repeat events
- Validates shortcuts (requires modifier or standalone F-key/fn)
- Shows warning for reserved system shortcuts
- Shows error for duplicates or invalid combinations
- Uses `isRecordingRef` and `recordedKeysRef` for synchronous access in async callbacks
- `saveInProgress` ref prevents duplicate concurrent saves

**Reset to Default**: Uses `resetBindings` (plural) backend command to atomically reset multiple shortcuts. See [hotkeyshortcut.md](file:///Users/tiger/Dev/opensource/speechGen/Handy/doc/hotkeyshortcut.md#reset-to-default).

**Reserved Shortcut Blocking** (backend, [shortcut.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/shortcut.rs)):
- macOS: `fn+a/c/d/e/f/h/m/n/q`, `cmd+c/v/x/z/a/s/n/o/p/w/q/h/m/tab/space`
- Windows: `super+l/d/e/r/tab`, `alt+tab/f4`, `ctrl+c/v/x/z/y/a/s/n/o/p/w`
- Linux: `alt+tab/f4`, `super+l/d`, `ctrl+c/v/x/z/y/a/s/n/o/p/w`

### 9. Language Select Step

**Component**: [LanguageSelectStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/LanguageSelectStep.tsx)

**Features**:
- Multi-language selection with one active language
- 100 Whisper-supported languages with emoji flags
- Searchable modal with 3-column grid layout
- Auto-detect toggle (uses Whisper's automatic language detection)
- Active language appears leftmost with highlighted style
- Clicking inactive language makes it active (reorders)
- Tooltip on hover: "Make this the default active language"

**Backend Settings**:
- `selected_language: String` - Active language code or "auto"
- `saved_languages: Vec<String>` - User's preferred language list

**Language Data**: [languageData.ts](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/lib/constants/languageData.ts)
- `WhisperLanguageCode` union type for compile-time safety
- All 100 Whisper languages with ISO 639-1 codes and emoji flags
- Helper functions: `getLanguageByCode`, `getLanguageFlag`, `getLanguageLabel` (returns `undefined` for "auto" to use i18n)

### 10. Learn Step

**Component**: [LearnStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/LearnStep.tsx)

Interactive Slack-style chat UI where users practice transcription hotkeys.

**Features**:
- Bot "Alex" with avatar greets user by name
- User avatar shows initials derived from `userName`
- Canned responses guide user through push-to-talk (`fn`) and hands-free (`fn+Space`) modes
- Typing indicator with 1.5-2.5s delay before bot responses
- Focused input field receives transcribed text from backend
- Skip button jumps to completion; Continue button available after conversation

**Paste Override Workaround**:
WebView doesn't receive CGEvent-simulated Cmd+V from the same process. Solution:
- `OnboardingPasteOverride` managed state in `lib.rs`
- `set_onboarding_paste_override` command called on mount/unmount
- `clipboard.rs` uses Direct paste (character-by-character) when override enabled

**Files**:
- Bot avatar: `/src-tauri/resources/botAvatar.png`
- i18n keys: `onboarding.learn.*`

### 11. Success Step

**Component**: [SuccessStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/SuccessStep.tsx)

**Features**:
- Displays Pro trial unlocked message with 2-week free trial badge
- Lists Pro features (unlimited transcriptions, AI text refinement, priority support)
- "No credit card required" message
- Continue button proceeds to Referral step

**i18n keys**: `onboarding.success.*`

### 12. Referral Step

**Component**: [ReferralStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/ReferralStep.tsx)

**Features**:
- Left panel: Title, "How it works?" with 3 steps, referral link input with Copy button
- Right panel: Share card with 3D hover animation (±10° rotation following cursor)
- Glare effect on hover that moves with cursor position
- Enhanced shadows for elevated card appearance
- Toast notification (bottom-right) when link is copied
- Finish button completes onboarding and exits to main app

**Visual Effects**:
- 3D perspective transform on card (1000px perspective)
- Multi-layer wave pattern with gradient
- Shimmer overlay effect
- Enhanced box-shadow with ambient glow

**i18n keys**: `onboarding.referral.*`

---

## Layout & Design

**Component**: [OnboardingLayout.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/OnboardingLayout.tsx)

**Structure**:
- Left panel (45%): Content, questions, inputs
- Right panel (55%): Illustrations, videos
- Progress bar at top

**Colors**:
- Dark mode: Left `#1E1E1E`, Right `#FFFDE8` (warm cream)
- Light mode: Left white, Right `#FBF5E5`

**Selection styling**: Border-highlight (`border-primary bg-primary/5`)

---

## State Management

**User Profile Hook**: [useUserProfile.ts](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/hooks/useUserProfile.ts)

**User Profile Store**: [userProfileStore.ts](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/stores/userProfileStore.ts)

**Persistence**: `~/Library/Application Support/com.pais.codictate/user_store.json`

---

## Backend: Graceful Permission Handling

**Problem**: App crashed if accessibility permissions not granted at startup.

**Solution**: Lazy Enigo initialization.

**Files modified**:
- [input.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/input.rs) - `EnigoState` now wraps `Option<Enigo>`
- [lib.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/lib.rs) - Removed `.expect()` panic
- [clipboard.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/clipboard.rs) - Calls `try_init()` before paste

**Behavior**:
- App starts without crashing even without permissions
- Enigo initializes automatically when permissions are granted
- Clear error message if paste attempted without permissions

---

## Translations

All strings in [translation.json](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/i18n/locales/en/translation.json):

- `onboarding.welcome.*`
- `onboarding.attribution.*`
- `onboarding.tellUsAboutYou.*`
- `onboarding.typingUseCases.*`
- `onboarding.permissions.*`
- `onboarding.downloadModel.*`
- `onboarding.microphoneCheck.*`
- `onboarding.hotkeySetup.*`
- `onboarding.languageSelect.*`
- `onboarding.learn.*`
- `onboarding.success.*`
- `onboarding.referral.*`

---

## Testing

```bash
# Reset onboarding state
rm ~/Library/Application\ Support/com.pais.codictate/user_store.json

# Reset macOS permissions (requires restart)
tccutil reset Accessibility com.pais.codictate
tccutil reset Microphone com.pais.codictate

# Trigger onboarding in-app
Cmd+Shift+O (macOS) / Ctrl+Shift+O (Windows/Linux)
```
