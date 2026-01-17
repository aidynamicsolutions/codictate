# Onboarding Flow Documentation

Comprehensive documentation of the Codictate onboarding experience.

## Flow Overview

```
Welcome → Attribution → Tell Us About You → Typing Use Cases → Permissions → Microphone Check → Hotkey Setup → Learn
   1           2                3                  4               5              6                 7            8
```

Steps 1-4 collect user profile data. Steps 5-8 configure the app.

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

### 6. Microphone Check Step

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

### 7. Hotkey Setup Step (Placeholder)

**Component**: [HotkeySetupStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/HotkeySetupStep.tsx)

**Planned features**:
- Display current shortcut binding
- Allow user to record new shortcut
- Visual feedback for shortcut capture

### 8. Learn Step (Placeholder)

**Component**: [LearnStep.tsx](file:///Users/tiger/Dev/opensource/speechGen/Handy/src/components/onboarding/LearnStep.tsx)

**Planned features**:
- Tutorial on app usage
- Demo of transcription flow

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
- `onboarding.setup.*`
- `onboarding.learn.*`

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
