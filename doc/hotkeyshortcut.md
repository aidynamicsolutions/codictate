# Hotkey Shortcut System

Concise documentation of Codictate's keyboard shortcut system for speech-to-text.

## Default Shortcuts (macOS)

| Action | Shortcut | Mode | Description |
|--------|----------|------|-------------|
| Push-to-talk | `fn` | Hold | Hold to record, release to transcribe |
| Hands-free | `fn+space` | Toggle | Press to start, press again to stop |
| Cancel | `esc` | Instant | Cancel current recording (auto-registered) |

## How It Works

### Mode Detection (150ms delay)

```
 User presses Fn
       │
       ▼
 ┌─────────────────┐
 │  Start 150ms    │
 │    timer        │
 └─────────────────┘
       │
       ├──── Space pressed within 150ms ──▶ HANDS-FREE MODE
       │                                    (toggle on/off)
       │
       └──── 150ms expires without Space ──▶ PUSH-TO-TALK MODE
                                             (hold to record)
```

### Push-to-Talk Flow

```
  Fn Down          150ms delay          Recording          Fn Up
     │                 │                   │                  │
     ▼                 ▼                   ▼                  ▼
 ┌───────┐        ┌─────────┐        ┌──────────┐       ┌───────────┐
 │ Start │  ───▶  │  Timer  │  ───▶  │ Overlay  │ ───▶  │Transcribe │
 │ timer │        │ expires │        │ appears  │       │   & type  │
 └───────┘        └─────────┘        └──────────┘       └───────────┘
```

### Hands-Free Flow

```
  fn+space          fn+space (again)
     │                   │
     ▼                   ▼
 ┌───────────┐      ┌───────────┐
 │  Toggle   │      │  Toggle   │
 │    ON     │ ───▶ │    OFF    │
 │ (record)  │      │(transcribe)│
 └───────────┘      └───────────┘
     │                   │
     ▼                   ▼
  Overlay            Overlay
  appears            disappears
```

### Late fn+space Detection

If user presses Space after the 150ms delay (PTT already started):

```
  Fn Down    ───▶    150ms    ───▶   PTT starts   ───▶  Space pressed
                                                             │
                                                             ▼
                                                      ┌─────────────┐
                                                      │ Cancel PTT  │
                                                      │ Start HF    │
                                                      └─────────────┘
```

### Key Bounce Handling (Debounce)

To prevent accidental recording stops due to key "bounces" (brief release-then-press events, common with mechanical keys or after system sleep), a **150ms debounce** is applied to the Fn key release.

```
  Fn Up            150ms debounce          Fn Down (bounce)
    │                    │                        │
    ▼                    ▼                        ▼
┌────────┐         ┌──────────┐             ┌──────────┐
│ Detect │  ───▶   │ Wait for │   ───▶      │ Release  │
│ Release│         │ confirm  │             │ cancelled│
└────────┘         └──────────┘             └──────────┘
                         │
                         ▼
                   (No bounced press)
                         │
                         ▼
                   ┌───────────┐
                   │ Stop PTT  │
                   └───────────┘
```

## Implementation

### Backend Files

| File | Purpose |
|------|---------|
| [fn_key_monitor.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/fn_key_monitor.rs) | CGEventTap-based Fn key detection |
| [shortcut.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/shortcut.rs) | Global shortcut registration |
| [actions.rs](file:///Users/tiger/Dev/opensource/speechGen/Handy/src-tauri/src/actions.rs) | `TranscribeAction` start/stop logic |

### Key State Variables

```rust
FN_KEY_WAS_PRESSED    // Tracks if Fn is currently held
FN_SPACE_TRIGGERED    // True if fn+space was used this session
PTT_STARTED           // True if push-to-talk recording started
FN_PRESS_COUNTER      // Invalidates stale timers on rapid presses
DEBOUNCING_RELEASE    // True if checking for key release bounce
RELEASE_DEBOUNCE_MS   // Debounce duration (150ms)
```

### Mutual Exclusivity

- When PTT starts → hands-free toggle state is reset to `false`
- When fn+space detected with PTT active → PTT is canceled first

## Reserved Shortcuts (Blocked)

### macOS
- `fn+a`, `fn+c`, `fn+d`, `fn+e`, `fn+f`, `fn+h`, `fn+m`, `fn+n`, `fn+q`
- Standard system shortcuts (Cmd+C, Cmd+V, etc.)

### Customization

Users can change shortcuts in Settings → Shortcuts. The app validates:
1. No conflicts with system shortcuts
2. No duplicates between push-to-talk and hands-free
3. Requires modifier key (except standalone Fn)

## Reset to Default

Use `resetBindings` (plural) to atomically reset multiple shortcuts at once. This bypasses duplicate checking between the shortcuts being reset.

```typescript
// ❌ Sequential resets can fail on conflicts
await resetBinding("transcribe");      // fails if transcribe_handsfree has "fn"
await resetBinding("transcribe_handsfree");

// ✅ Atomic reset handles any combination
await resetBindings(["transcribe", "transcribe_handsfree"]);
```

## React Best Practices (Lessons Learned)

The `useShortcutRecorder` hook follows these patterns:

1. **Use refs for async callbacks** - React state can be stale in closures. Use `useRef` to track values that async callbacks need to access synchronously.
2. **Avoid nested setState** - Don't call async functions from within `setState` updaters.
3. **Guard duplicate calls** - Use a `saveInProgress` ref to prevent concurrent save operations.
4. **Keep refs in sync with state** - When you need both reactive updates AND synchronous access, update both.

## Shortcut Recording

When recording a new shortcut, transcription triggers are disabled to prevent interference:

```
  User clicks to record
         │
         ▼
  ┌─────────────────────────────────┐
  │ 1. await startFnKeyMonitor(false) │  ← Disable Fn transcription (awaited!)
  │ 2. suspendBinding(shortcutId)     │  ← Suspend global hotkey
  └─────────────────────────────────┘
         │
         ▼
     Recording Mode
     (user presses keys)
         │
         ▼
  ┌─────────────────────────────────┐
  │ 1. startFnKeyMonitor(true)        │  ← Re-enable Fn transcription
  │ 2. resumeBinding(shortcutId)      │  ← Resume global hotkey
  └─────────────────────────────────┘
```

**Key insight**: The `startFnKeyMonitor(false)` call must be **awaited** before entering recording mode. Fire-and-forget allows race conditions where fn+space triggers transcription before disabling completes.

## Thread Safety (Rust)

The Fn key monitor uses `OnceLock<Mutex<Option<T>>>` for thread-safe state:

```rust
// Thread-safe app handle storage
static APP_HANDLE: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

// Helper functions for safe access
fn get_app_handle() -> Option<AppHandle> { ... }
fn set_app_handle(handle: Option<AppHandle>) { ... }
```

The run loop is also stored to enable proper cleanup when stopping the monitor.
