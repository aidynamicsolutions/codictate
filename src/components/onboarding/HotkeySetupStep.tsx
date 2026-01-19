import React, { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Pencil } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { type } from "@tauri-apps/plugin-os";
import { Button } from "@/components/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/shared/ui/dialog";
import OnboardingLayout from "./OnboardingLayout";
import FnKeyVisual from "./FnKeyVisual";
import { useSettings } from "@/hooks/useSettings";
import { useShortcutRecorder } from "@/hooks/useShortcutRecorder";
import { commands } from "@/bindings";
import { logError } from "@/utils/logging";

// Helper component to render individual key badge
const KeyBadge: React.FC<{ keyName: string }> = ({ keyName }) => {
  // Capitalize first letter for display
  const displayName = keyName.charAt(0).toUpperCase() + keyName.slice(1);
  return (
    <span className="inline-flex items-center justify-center px-2 py-1 text-sm font-medium bg-muted border border-border rounded min-w-[36px]">
      {displayName}
    </span>
  );
};

// Component to display a shortcut binding with styled key badges
interface ShortcutCardProps {
  shortcutId: string;
  title: string;
  description: string;
  /** Key to force re-mount (cancels active recording) */
  resetKey?: number;
}

const ShortcutCard: React.FC<ShortcutCardProps> = ({
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

  // Use the shared shortcut recorder hook
  const { isRecording, displayKeys, startRecording, error, warning, clearError } = useShortcutRecorder({
    onSave: async (shortcut) => {
      await updateBinding(shortcutId, shortcut);
    },
    onCancel: () => {
      // Resume the suspended binding on cancel
      commands.resumeBinding(shortcutId).catch((err) =>
        logError(`Failed to resume binding: ${err}`, "fe-onboarding")
      );
    },
    onRecordingStart: () => {
      // Suspend the binding while recording to avoid triggering transcription
      commands.suspendBinding(shortcutId).catch((err) =>
        logError(`Failed to suspend binding: ${err}`, "fe-onboarding")
      );
    },
    onRecordingEnd: () => {
      // Resume the binding after recording completes successfully
      commands.resumeBinding(shortcutId).catch((err) =>
        logError(`Failed to resume binding: ${err}`, "fe-onboarding")
      );
    },
    requireModifier: true,
    containerRef,
  });

  // Clear error when input is reset to "Press keys..." state
  useEffect(() => {
    if (isRecording && displayKeys.length === 0 && error) {
      clearError();
    }
  }, [isRecording, displayKeys.length, error, clearError]);

  return (
    <div className="flex items-center justify-between p-5 border border-border rounded-lg select-none cursor-default">
      <div className="flex flex-col gap-1">
        <span className="text-sm font-semibold text-foreground">{title}</span>
        <span className="text-sm text-muted-foreground">{description}</span>
      </div>
      <div className="flex flex-col items-end gap-1.5">
        {/* Spacer to reserve space above the input */}
        <div className="h-4" />
        <button
          ref={containerRef}
          type="button"
          onClick={startRecording}
          className="flex items-center justify-between gap-2 px-3 py-2 min-w-[280px] min-h-[44px] bg-muted/50 border border-border hover:bg-muted rounded cursor-pointer hover:border-primary/50 transition-colors"
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
              <Pencil className="h-3.5 w-3.5 text-muted-foreground" />
            </>
          )}
        </button>
        {/* Fixed height container for error/warning messages */}
        <div className="h-4 flex items-center justify-end">
          {error && (
            <span className="text-xs text-destructive select-none">{error}</span>
          )}
          {warning && !error && (
            <span className="text-xs text-yellow-600 dark:text-yellow-500 select-none">{warning}</span>
          )}
        </div>
      </div>
    </div>
  );
};


interface HotkeySetupStepProps {
  onContinue: () => void;
  onBack: () => void;
}

export const HotkeySetupStep: React.FC<HotkeySetupStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();
  const { resetBindings } = useSettings();
  const [isKeyPressed, setIsKeyPressed] = useState(false);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isMacOS, setIsMacOS] = useState(false);
  const [resetKey, setResetKey] = useState(0);

  // Detect OS type
  useEffect(() => {
    const osType = type();
    setIsMacOS(osType === "macos");
  }, []);

  // On macOS, use native Fn key monitoring via CGEventTap
  useEffect(() => {
    if (!isMacOS) return;

    let unlistenFnDown: (() => void) | undefined;
    let unlistenFnUp: (() => void) | undefined;

    const startMonitoring = async () => {
      try {
        // Start the native Fn key monitor with transcription disabled (visual feedback only)
        await commands.startFnKeyMonitor(false);

        // Listen for fn-key-down event from Rust backend
        unlistenFnDown = await listen("fn-key-down", () => {
          setIsKeyPressed(true);
        });

        // Listen for fn-key-up event from Rust backend
        unlistenFnUp = await listen("fn-key-up", () => {
          setIsKeyPressed(false);
        });
      } catch (error) {
        logError(`Failed to start Fn key monitoring: ${error}`, "fe-onboarding");
      }
    };

    startMonitoring();

    return () => {
      // Cleanup: re-enable transcription on the Fn key monitor (not stopping it)
      commands.startFnKeyMonitor(true).catch((err) => 
        logError(`Failed to re-enable Fn transcription: ${err}`, "fe-onboarding")
      );
      unlistenFnDown?.();
      unlistenFnUp?.();
    };
  }, [isMacOS]);

  // On non-macOS, fallback to detecting modifier key presses
  useEffect(() => {
    if (isMacOS) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Show visual feedback when modifier keys or space are pressed
      if (e.metaKey || e.altKey || e.ctrlKey || e.key === " ") {
        setIsKeyPressed(true);
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      // Only reset when all modifier keys are released
      if (!e.metaKey && !e.altKey && !e.ctrlKey) {
        setIsKeyPressed(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
    };
  }, [isMacOS]);

  const handleResetToDefault = useCallback(async () => {
    // Increment resetKey first to cancel any active recording (causes ShortcutCard re-mount)
    setResetKey((prev) => prev + 1);
    
    try {
      // Use atomic reset that bypasses duplicate checking between the bindings
      // This handles any combination of conflicts (e.g., one set to the other's default)
      await resetBindings(["transcribe", "transcribe_handsfree"]);
    } catch (error) {
      logError(`Failed to reset bindings: ${error}`, "fe-onboarding");
    }
  }, [resetBindings]);

  const handleContinue = () => {
    onContinue();
  };

  const handleBack = () => {
    onBack();
  };

  return (
    <OnboardingLayout
      currentStep="hotkeySetup"
      leftContent={
        <div className="flex flex-col h-full">
          {/* Back button - positioned at top */}
          <button
            type="button"
            onClick={handleBack}
            className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors w-fit mb-auto"
          >
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.hotkeySetup.back")}
          </button>

          {/* Content centered vertically */}
          <div className="flex flex-col gap-4 my-auto">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl max-w-[380px]">
              {t("onboarding.hotkeySetup.title")}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.hotkeySetup.subtitle")}{" "}
              <span className="inline-block px-1.5 py-0.5 bg-muted rounded text-foreground font-medium text-sm border border-border">
                {t("onboarding.hotkeySetup.subtitleFnKey")}
              </span>{" "}
              {t("onboarding.hotkeySetup.subtitleEnd")}
            </p>
          </div>

          {/* Spacer to balance layout */}
          <div className="mb-auto" />
        </div>
      }
      rightContent={
        <div className="flex items-center justify-center h-full w-full">
          {/* Main card */}
          <div className="bg-background rounded-xl shadow-lg p-8 max-w-md w-full">
            <div className="flex flex-col gap-6">
              {/* Question */}
              <p className="text-center text-foreground font-medium">
                {t("onboarding.hotkeySetup.question")}
              </p>

              {/* Fn Key visualization area */}
              <div className="bg-accent/30 rounded-lg p-8 flex items-center justify-center">
                <FnKeyVisual isPressed={isKeyPressed} />
              </div>

              {/* Action buttons */}
              <div className="flex items-center justify-center gap-3">
                <Button variant="outline" onClick={() => setIsModalOpen(true)}>
                  {t("onboarding.hotkeySetup.changeShortcut")}
                </Button>

                <Button onClick={handleContinue}>
                  {t("onboarding.hotkeySetup.yes")}
                </Button>
              </div>
            </div>
          </div>

          {/* Change Hotkeys Modal */}
          <Dialog open={isModalOpen} onOpenChange={setIsModalOpen}>
            <DialogContent className="sm:max-w-[700px] select-none cursor-default">
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
        </div>
      }
    />
  );
};

export default HotkeySetupStep;
