import React, { useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Pencil, Check } from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/shared/ui/dialog";
import { useSettings } from "@/hooks/useSettings";
import { useShortcutRecorder } from "@/hooks/useShortcutRecorder";
import { commands } from "@/bindings";
import { logError } from "@/utils/logging";

// macOS modifier key symbols mapping (only for allowed modifier keys)
const MAC_KEY_SYMBOLS: Record<string, string> = {
  command: "⌘",
  cmd: "⌘",
  option: "⌥",
  opt: "⌥",
  alt: "⌥",
  control: "⌃",
  ctrl: "⌃",
  shift: "⇧",
};

/**
 * Helper to get the display symbol and text for a key
 */
export const getKeyDisplay = (
  keyName: string
): { symbol: string | null; text: string } => {
  const normalizedKey = keyName.toLowerCase().trim();
  const symbol = MAC_KEY_SYMBOLS[normalizedKey] || null;

  // Keep "fn" and single letters lowercase, capitalize multi-character key names
  let text: string;
  if (normalizedKey === "fn" || normalizedKey.length === 1) {
    text = normalizedKey;
  } else {
    text = keyName.charAt(0).toUpperCase() + keyName.slice(1);
  }

  return { symbol, text };
};

/**
 * Component to render an individual key badge
 */
export const KeyBadge: React.FC<{ keyName: string }> = ({ keyName }) => {
  const { symbol, text } = getKeyDisplay(keyName);

  return (
    <span className="inline-flex items-center justify-center gap-1 px-2 py-1 text-sm font-medium bg-secondary/80 dark:bg-secondary border border-border/60 dark:border-border rounded min-w-[36px]">
      {symbol && <span className="text-muted-foreground">{symbol}</span>}
      <span>{text}</span>
    </span>
  );
};

/**
 * Props for ShortcutCard component
 */
interface ShortcutCardProps {
  shortcutId: string;
  title: string;
  description: string;
  /** Key to force re-mount (cancels active recording) */
  resetKey?: number;
}

/**
 * Component to display a shortcut binding with styled key badges
 */
export const ShortcutCard: React.FC<ShortcutCardProps> = ({
  shortcutId,
  title,
  description,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateBinding } = useSettings();
  const containerRef = useRef<HTMLButtonElement>(null);

  const bindings = getSetting("bindings") || {};
  const binding = bindings[shortcutId];
  const currentBinding = binding?.current_binding || "";

  // Parse the binding string into individual keys
  const parseBinding = (bindingStr: string): string[] => {
    if (!bindingStr) return [];
    return bindingStr.split("+").map((key) => key.trim());
  };

  const keys = parseBinding(currentBinding);

  const [showSuccess, setShowSuccess] = React.useState(false);

  // Use the shared shortcut recorder hook
  const { isRecording, displayKeys, startRecording, error, warning, clearError } = useShortcutRecorder({
    onSave: async (shortcut) => {
      await updateBinding(shortcutId, shortcut);
      setShowSuccess(true);
      toast.success(t("settings.general.shortcut.success", "Shortcut saved"));
      setTimeout(() => setShowSuccess(false), 2000);
    },
    onCancel: () => {
      // Resume the suspended binding on cancel
      commands.resumeBinding(shortcutId).catch((err) =>
        logError(`Failed to resume binding: ${err}`, "fe-shortcuts")
      );
    },
    onRecordingStart: () => {
      // Suspend the binding while recording to avoid triggering transcription
      commands.suspendBinding(shortcutId).catch((err) =>
        logError(`Failed to suspend binding: ${err}`, "fe-shortcuts")
      );
    },
    onRecordingEnd: () => {
      // Note: We do NOT call resumeBinding() here because change_binding()
      // already registers the new shortcut. Calling resumeBinding() would
      // cause "already in use" errors.
    },
    requireModifier: true,
    containerRef,
    t,
  });

  // Clear error when input is reset to "Press keys..." state
  useEffect(() => {
    if (isRecording && displayKeys.length === 0 && error) {
      clearError();
    }
  }, [isRecording, displayKeys.length, error, clearError]);

  return (
    <div className="flex items-center justify-between gap-4 px-4 py-1 border border-border/50 dark:border-border dark:bg-card/50 rounded-lg select-none cursor-default">
      <div className="flex flex-col gap-0.5 max-w-[300px]">
        <span className="text-base font-medium text-foreground">{title}</span>
        <span className="text-sm text-muted-foreground">{description}</span>
      </div>
      <div className="flex flex-col items-end gap-1 flex-shrink-0">
        {/* Spacer to vertically center the input with the title */}
        <div className="h-5" />
        <button
          ref={containerRef}
          type="button"
          onClick={startRecording}
          className="flex items-center justify-between gap-2 px-3 py-1.5 min-w-[280px] min-h-[40px] bg-muted/50 dark:bg-muted/30 border border-border/60 dark:border-border hover:bg-muted dark:hover:bg-muted/50 rounded cursor-pointer hover:border-primary/50 transition-colors"
        >
          {isRecording ? (
            <>
              <div className="flex items-center gap-1">
                {displayKeys.length > 0 ? (
                  displayKeys.map((key, i) => (
                    <KeyBadge key={i} keyName={key} />
                  ))
                ) : (
                  <span className="text-sm text-muted-foreground">
                    {t("onboarding.hotkeySetup.modal.pressKeys", "Press keys...")}
                  </span>
                )}
              </div>
              <Pencil className="h-3.5 w-3.5 text-muted-foreground" />
            </>
          ) : (
            <>
              <div className="flex items-center gap-1">
                {keys.map((key, index) => (
                  <KeyBadge key={index} keyName={key} />
                ))}
              </div>
              {showSuccess ? (
                <Check className="h-3.5 w-3.5 text-green-500" />
              ) : (
                <Pencil className="h-3.5 w-3.5 text-muted-foreground" />
              )}
            </>
          )}
        </button>
        {/* Fixed height container for error/warning messages - sized for 3 lines to prevent card resizing */}
        <div className="h-4 max-w-[280px] mt-1 mb-1 flex items-start justify-end">
          {error && (
            <span className="text-xs text-destructive select-none leading-tight text-right">{error}</span>
          )}
          {warning && !error && (
            <span className="text-xs text-yellow-600 dark:text-yellow-500 select-none leading-tight text-right">{warning}</span>
          )}
        </div>
      </div>
    </div>
  );
};

/**
 * Props for KeyboardShortcutsModal
 */
interface KeyboardShortcutsModalProps {
  /** Whether the modal is open */
  open: boolean;
  /** Handler to change the open state */
  onOpenChange: (open: boolean) => void;
}

/**
 * Shared modal component for configuring keyboard shortcuts.
 * Used in both Settings page and Onboarding flow.
 */
export const KeyboardShortcutsModal: React.FC<KeyboardShortcutsModalProps> = ({
  open,
  onOpenChange,
}) => {
  const { t } = useTranslation();
  const { resetBindings } = useSettings();
  const [resetKey, setResetKey] = React.useState(0);

  const handleResetToDefault = useCallback(async () => {
    // Increment resetKey first to cancel any active recording (causes ShortcutCard re-mount)
    setResetKey((prev) => prev + 1);

    try {
      // Use atomic reset that bypasses duplicate checking between the bindings
      // This handles any combination of conflicts (e.g., one set to the other's default)
      await resetBindings(["transcribe", "transcribe_handsfree", "paste_last_transcript", "refine_last_transcript"]);
    } catch (error) {
      logError(`Failed to reset bindings: ${error}`, "fe-shortcuts");
    }
  }, [resetBindings]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[700px] max-h-[85vh] overflow-y-auto select-none cursor-default border-border/60 shadow-2xl dark:border-border dark:shadow-black/50 dark:bg-card">
        <DialogHeader className="mb-4">
          <DialogTitle>
            {t("onboarding.hotkeySetup.modal.title")}
          </DialogTitle>
          <DialogDescription>
            {t("onboarding.hotkeySetup.modal.subtitle", {
              appName: t("appName"),
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-5 mt-2">
          {/* Push to talk shortcut */}
          <ShortcutCard
            key={`transcribe-${resetKey}`}
            shortcutId="transcribe"
            title={t("settings.general.shortcut.bindings.transcribe.name")}
            description={t("settings.general.shortcut.bindings.transcribe.description")}
          />

          {/* Hands-free mode shortcut */}
          <ShortcutCard
            key={`transcribe_handsfree-${resetKey}`}
            shortcutId="transcribe_handsfree"
            title={t("settings.general.shortcut.bindings.transcribe_handsfree.name")}
            description={t("settings.general.shortcut.bindings.transcribe_handsfree.description")}
          />

          {/* Paste last transcript shortcut */}
          <ShortcutCard
            key={`paste_last_transcript-${resetKey}`}
            shortcutId="paste_last_transcript"
            title={t("settings.general.shortcut.bindings.paste_last_transcript.name")}
            description={t("settings.general.shortcut.bindings.paste_last_transcript.description")}
          />

          {/* Refine last transcript shortcut */}
          <ShortcutCard
            key={`refine_last_transcript-${resetKey}`}
            shortcutId="refine_last_transcript"
            title={t("settings.general.shortcut.bindings.refine_last_transcript.name")}
            description={t("settings.general.shortcut.bindings.refine_last_transcript.description")}
          />

          {/* Divider */}
          <div className="border-t border-border mt-2" />

          {/* Reset to default */}
          <button
            type="button"
            onClick={handleResetToDefault}
            className="text-sm text-muted-foreground hover:text-foreground transition-colors text-center py-2"
          >
            {t("onboarding.hotkeySetup.modal.resetToDefault")}
          </button>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default KeyboardShortcutsModal;
