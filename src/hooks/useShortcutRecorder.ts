import { useState, useEffect, useCallback, useRef, RefObject } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { type } from "@tauri-apps/plugin-os";
import { getKeyName, normalizeKey, type OSType } from "@/lib/utils/keyboard";
import { logError } from "@/utils/logging";
import { commands } from "@/bindings";

/**
 * List of modifier keys (lowercase)
 */
const MODIFIERS = [
  "ctrl",
  "control",
  "shift",
  "alt",
  "option",
  "meta",
  "command",
  "cmd",
  "super",
  "win",
  "windows",
  "fn",
];

/**
 * Keys that are allowed without a modifier (function keys, etc.)
 */
const STANDALONE_ALLOWED = [
  "fn",
  "escape",
  "esc",
  "f1",
  "f2",
  "f3",
  "f4",
  "f5",
  "f6",
  "f7",
  "f8",
  "f9",
  "f10",
  "f11",
  "f12",
  "f13",
  "f14",
  "f15",
  "f16",
  "f17",
  "f18",
  "f19",
  "f20",
];

/**
 * Reserved OS shortcuts that may conflict with system functionality
 */
const RESERVED_SHORTCUTS: Record<string, string[]> = {
  macos: [
    // Fn/Globe key combinations (system-reserved)
    "fn+a",      // Show/hide Dock
    "fn+c",      // Control Center
    "fn+d",      // Dictation
    "fn+e",      // Emoji picker
    "fn+f",      // Full screen
    "fn+h",      // Show desktop
    "fn+m",      // Focus menu bar
    "fn+n",      // Notification Center
    "fn+q",      // Quick Note
    // Command shortcuts (critical)
    "command+space",    // Spotlight
    "command+tab",      // App switcher
    "command+q",        // Quit app
    "command+w",        // Close window
    "command+h",        // Hide app
    "command+m",        // Minimize
    "command+c",        // Copy
    "command+v",        // Paste
    "command+x",        // Cut
    "command+z",        // Undo
    "command+a",        // Select all
    "command+s",        // Save
    "command+n",        // New
    "command+o",        // Open
    "command+p",        // Print
    // System
    "command+option+escape",  // Force Quit
    "control+command+q",      // Lock screen
    "shift+command+3",        // Screenshot full
    "shift+command+4",        // Screenshot area
    "shift+command+5",        // Screenshot menu
  ],
  windows: [
    "win+l",            // Lock
    "win+d",            // Desktop
    "win+e",            // File Explorer
    "win+r",            // Run
    "win+tab",          // Task View
    "alt+tab",          // App switcher
    "alt+f4",           // Close app
    "ctrl+alt+delete",  // Security screen
    "ctrl+c",           // Copy
    "ctrl+v",           // Paste
    "ctrl+x",           // Cut
    "ctrl+z",           // Undo
    "ctrl+a",           // Select all
    "ctrl+s",           // Save
    "ctrl+n",           // New
    "ctrl+o",           // Open
    "ctrl+p",           // Print
  ],
  linux: [
    "alt+tab",          // App switcher
    "alt+f4",           // Close app
    "ctrl+c",           // Copy
    "ctrl+v",           // Paste
    "ctrl+x",           // Cut
    "ctrl+z",           // Undo
    "ctrl+a",           // Select all
    "ctrl+s",           // Save
  ],
  unknown: [],
};

interface UseShortcutRecorderOptions {
  /** Callback when a valid shortcut is recorded */
  onSave: (shortcut: string) => Promise<void>;
  /** Callback when recording is cancelled (e.g., ESC pressed) */
  onCancel?: () => void;
  /** Callback when recording starts (e.g., to suspend binding) */
  onRecordingStart?: () => void;
  /** Callback when recording ends (success or cancel) */
  onRecordingEnd?: () => void;
  /** Whether to require at least one modifier key (default: true) */
  requireModifier?: boolean;
  /** Container element ref for click-outside detection */
  containerRef?: RefObject<HTMLElement | null>;
  /** Optional translation function for error messages (accepts key and returns translated string) */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  t?: (key: string, defaultValue?: any) => string;
}

interface UseShortcutRecorderReturn {
  /** Whether currently recording a shortcut */
  isRecording: boolean;
  /** Keys to display during recording */
  displayKeys: string[];
  /** Start recording a new shortcut */
  startRecording: () => void;
  /** Stop recording without saving */
  stopRecording: () => void;
  /** Error message if validation fails */
  error: string | null;
  /** Warning message for reserved shortcuts (shortcut still saved) */
  warning: string | null;
  /** Clear error message */
  clearError: () => void;
  /** Clear warning message */
  clearWarning: () => void;
}

/**
 * Hook for recording keyboard shortcuts with proper handling for:
 * - ESC to cancel
 * - Auto-repeat filtering
 * - Modifier sorting
 * - Validation (require modifier key)
 * - Native Fn key support on macOS via Tauri events
 * - Click outside to cancel
 */
export function useShortcutRecorder(
  options: UseShortcutRecorderOptions
): UseShortcutRecorderReturn {
  const {
    onSave,
    onCancel,
    onRecordingStart,
    onRecordingEnd,
    requireModifier = true,
    containerRef,
    t,
  } = options;

  // Helper to get translated message or fallback
  const getMessage = useCallback((key: string, fallback: string) => {
    return t ? t(`settings.general.shortcut.errors.${key}`, fallback) : fallback;
  }, [t]);

  const [isRecording, setIsRecording] = useState(false);
  const [keyPressed, setKeyPressed] = useState<string[]>([]); // Currently held keys
  const [recordedKeys, setRecordedKeys] = useState<string[]>([]); // All keys in combo
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [osType, setOsType] = useState<OSType>("unknown");

  // Track Fn key state for macOS
  const fnKeyPressed = useRef(false);
  // Guard to prevent duplicate saves (React setState updaters can fire multiple times)
  const saveInProgress = useRef(false);
  // Track recording state synchronously for async callbacks (React state can be stale in closures)
  const isRecordingRef = useRef(false);
  // Track recorded keys synchronously for async callbacks (avoids nested setState calls)
  const recordedKeysRef = useRef<string[]>([]);

  // Detect OS on mount
  useEffect(() => {
    const detectedType = type();
    if (detectedType === "macos") {
      setOsType("macos");
    } else if (detectedType === "windows") {
      setOsType("windows");
    } else if (detectedType === "linux") {
      setOsType("linux");
    }
  }, []);

  // Sort keys: modifiers first, then main keys
  const sortKeys = useCallback((keys: string[]): string[] => {
    return [...keys].sort((a, b) => {
      const aIsModifier = MODIFIERS.includes(a.toLowerCase());
      const bIsModifier = MODIFIERS.includes(b.toLowerCase());
      if (aIsModifier && !bIsModifier) return -1;
      if (!aIsModifier && bIsModifier) return 1;
      return 0;
    });
  }, []);

  // Validate the shortcut
  const validateShortcut = useCallback(
    (keys: string[]): string | null => {
      if (keys.length === 0) {
        return getMessage("noKeysRecorded", "No keys recorded");
      }

      if (!requireModifier) {
        return null; // Skip validation
      }

      // Check if there's at least one modifier OR it's a standalone allowed key
      const hasModifier = keys.some((k) =>
        MODIFIERS.includes(k.toLowerCase())
      );
      const isStandaloneAllowed =
        keys.length === 1 &&
        STANDALONE_ALLOWED.includes(keys[0].toLowerCase());

      // Count non-modifier keys
      const nonModifierKeys = keys.filter(
        (k) => !MODIFIERS.includes(k.toLowerCase())
      );

      // Check for multiple non-modifier keys (e.g., Ctrl+Y+U)
      // OS-level global shortcuts only support one non-modifier key
      if (nonModifierKeys.length > 1) {
        return getMessage("multipleNonModifierKeys", "Shortcuts can only have one main key. Use a modifier (Ctrl, Shift, Alt, Cmd) with a single key.");
      }

      // Modifiers alone (e.g., just "Command") are not valid shortcuts
      // Must have either: a standalone allowed key, OR a modifier + non-modifier key
      if (!isStandaloneAllowed && hasModifier && nonModifierKeys.length === 0) {
        return getMessage("modifierRequired", "Shortcuts must include a modifier key (Ctrl, Shift, Alt, Cmd) or be a function key");
      }

      if (!hasModifier && !isStandaloneAllowed) {
        return getMessage("modifierRequired", "Shortcuts must include a modifier key (Ctrl, Shift, Alt, Cmd) or be a function key");
      }

      return null;
    },
    [requireModifier, getMessage]
  );

  const startRecording = useCallback(async () => {
    // Disable Fn key transcription BEFORE entering recording mode
    // This must complete before we allow key input to prevent fn+space from triggering
    if (osType === "macos") {
      try {
        await commands.startFnKeyMonitor(false);
      } catch (err) {
        logError(`Failed to disable Fn transcription: ${err}`, "fe-shortcuts");
      }
    }
    
    // THEN: Enter recording mode after transcription is disabled
    setIsRecording(true);
    isRecordingRef.current = true;
    setKeyPressed([]);
    setRecordedKeys([]);
    recordedKeysRef.current = [];
    setError(null);
    setWarning(null);
    fnKeyPressed.current = false;
    saveInProgress.current = false;
    
    onRecordingStart?.();
  }, [onRecordingStart, osType]);

  const stopRecording = useCallback(() => {
    setIsRecording(false);
    isRecordingRef.current = false;
    setKeyPressed([]);
    setRecordedKeys([]);
    recordedKeysRef.current = [];
    setError(null);
    setWarning(null);
    fnKeyPressed.current = false;
    saveInProgress.current = false;
    
    // Re-enable Fn key transcription after recording completes
    if (osType === "macos") {
      commands.startFnKeyMonitor(true).catch((err) => 
        logError(`Failed to re-enable Fn transcription: ${err}`, "fe-shortcuts")
      );
    }
    
    onRecordingEnd?.();
  }, [onRecordingEnd, osType]);

  const cancelRecording = useCallback(() => {
    stopRecording();
    onCancel?.();
  }, [stopRecording, onCancel]);

  // Function to commit a shortcut when all keys are released
  const tryCommitShortcut = useCallback(async (
    currentRecordedKeys: string[],
    currentKeyPressed: string[],
    fnPressed: boolean
  ) => {
    // Guard 1: Check if we're still recording (use ref for synchronous check in async context)
    if (!isRecordingRef.current) {
      return;
    }

    // Guard 2: Prevent duplicate calls
    if (saveInProgress.current) {
      return;
    }

    // Check if all keys are released
    const allReleased = currentKeyPressed.length === 0 && !fnPressed;


    if (allReleased && currentRecordedKeys.length > 0) {
      // Validate the shortcut
      const validationError = validateShortcut(currentRecordedKeys);

      if (validationError) {
        setError(validationError);
        setTimeout(() => {
          setRecordedKeys([]);
          setError(null);
        }, 5000);
        return;
      }

      // Sort keys and create shortcut string
      const sortedKeys = sortKeys(currentRecordedKeys);
      const shortcut = sortedKeys.join("+").toLowerCase();


      // Mark save as in progress to prevent duplicate calls
      saveInProgress.current = true;

      // Check if shortcut is reserved (warning, not error)
      const reservedList = RESERVED_SHORTCUTS[osType] || [];
      const isReserved = reservedList.includes(shortcut);
      
      if (isReserved) {
        setWarning("This shortcut may conflict with system shortcuts");
      } else {
        setWarning(null);
      }

      try {
        await onSave(shortcut);
        stopRecording();
      } catch (err) {
        logError(`Failed to save shortcut: ${err}`, "fe-shortcuts");
        // Parse error message for better user feedback
        const errorStr = String(err);
        if (errorStr.includes("already in use") || errorStr.includes("already registered")) {
          setError("This shortcut is already in use");
        } else if (errorStr.includes("reserved by the system")) {
          setError("This shortcut is reserved by the system");
        } else if (errorStr.includes("Failed to parse") || errorStr.includes("invalid")) {
          setError("Invalid shortcut combination");
        } else {
          setError("Failed to save shortcut");
        }
        // Reset saveInProgress on error so user can try again
        saveInProgress.current = false;
      }
    }
  }, [validateShortcut, sortKeys, osType, onSave, stopRecording]);

  // Handle Fn key via Tauri events (macOS only)
  useEffect(() => {
    if (!isRecording || osType !== "macos") return;

    let unlistenDown: UnlistenFn | undefined;
    let unlistenUp: UnlistenFn | undefined;

    const setupListeners = async () => {
      unlistenDown = await listen("fn-key-down", () => {
        if (!fnKeyPressed.current) {
          fnKeyPressed.current = true;
          setKeyPressed((prev) => (prev.includes("fn") ? prev : [...prev, "fn"]));
          setRecordedKeys((prev) => {
            if (prev.includes("fn")) return prev;
            const newKeys = [...prev, "fn"];
            recordedKeysRef.current = newKeys;
            return newKeys;
          });
        }
      });

      unlistenUp = await listen("fn-key-up", () => {
        fnKeyPressed.current = false;
        // Update pressed keys immediately
        setKeyPressed((prev) => prev.filter((k) => k !== "fn"));
        
        // Use refs for synchronous access - no nested setState calls needed
        // This follows React best practices for accessing values in async callbacks
        if (isRecordingRef.current && recordedKeysRef.current.length > 0) {
          // Capture the recorded keys synchronously from the ref
          const keysToCommit = [...recordedKeysRef.current];
          
          // Schedule the commit for after React state updates complete
          setTimeout(() => {
            // Double-check we're still recording when the timeout fires
            if (isRecordingRef.current) {
              tryCommitShortcut(keysToCommit, [], false);
            }
          }, 10);
        }
      });
    };

    setupListeners();

    return () => {
      unlistenDown?.();
      unlistenUp?.();
    };
  }, [isRecording, osType, tryCommitShortcut]);

  // Handle click outside
  useEffect(() => {
    if (!isRecording || !containerRef?.current) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        cancelRecording();
      }
    };

    // Use setTimeout to avoid immediate trigger from the click that started recording
    const timeoutId = setTimeout(() => {
      window.addEventListener("click", handleClickOutside);
    }, 0);

    return () => {
      clearTimeout(timeoutId);
      window.removeEventListener("click", handleClickOutside);
    };
  }, [isRecording, containerRef, cancelRecording]);

  // Handle keyboard events
  useEffect(() => {
    if (!isRecording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Ignore auto-repeat
      if (e.repeat) return;

      // ESC cancels recording
      if (e.key === "Escape") {
        e.preventDefault();
        cancelRecording();
        return;
      }

      e.preventDefault();

      // Get OS-specific key name
      const rawKey = getKeyName(e, osType);
      const key = normalizeKey(rawKey);

      // Add to pressed keys
      if (!keyPressed.includes(key)) {
        setKeyPressed((prev) => [...prev, key]);
      }

      // Add to recorded keys (update both state and ref)
      if (!recordedKeys.includes(key)) {
        setRecordedKeys((prev) => {
          const newKeys = [...prev, key];
          recordedKeysRef.current = newKeys;
          return newKeys;
        });
      }
    };

    const handleKeyUp = async (e: KeyboardEvent) => {
      e.preventDefault();

      const rawKey = getKeyName(e, osType);
      const key = normalizeKey(rawKey);

      // Remove from pressed keys
      const updatedPressed = keyPressed.filter((k) => k !== key);
      setKeyPressed(updatedPressed);

      // Use the shared tryCommitShortcut function (ensures saveInProgress guard applies)
      await tryCommitShortcut(recordedKeys, updatedPressed, fnKeyPressed.current);
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
    };
  }, [
    isRecording,
    keyPressed,
    recordedKeys,
    osType,
    cancelRecording,
    tryCommitShortcut,
  ]);

  // Display keys: show recorded keys sorted
  const displayKeys = sortKeys(recordedKeys);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const clearWarning = useCallback(() => {
    setWarning(null);
  }, []);

  return {
    isRecording,
    displayKeys,
    startRecording,
    stopRecording,
    error,
    warning,
    clearError,
    clearWarning,
  };
}

