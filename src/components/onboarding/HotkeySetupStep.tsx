import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { type } from "@tauri-apps/plugin-os";
import { Button } from "@/components/shared/ui/button";
import OnboardingLayout from "./OnboardingLayout";
import FnKeyVisual from "./FnKeyVisual";
import { KeyboardShortcutsModal } from "@/components/shared/KeyboardShortcutsModal";
import { commands } from "@/bindings";
import { logError } from "@/utils/logging";

interface HotkeySetupStepProps {
  onContinue: () => void;
  onBack: () => void;
}

export const HotkeySetupStep: React.FC<HotkeySetupStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();
  const [isKeyPressed, setIsKeyPressed] = useState(false);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isMacOS, setIsMacOS] = useState(false);

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
              <div className="flex items-center justify-end gap-3">
                <Button variant="outline" onClick={() => setIsModalOpen(true)}>
                  {t("onboarding.hotkeySetup.changeShortcut")}
                </Button>

                <Button onClick={handleContinue} className="min-w-[80px]">
                  {t("onboarding.hotkeySetup.yes")}
                </Button>
              </div>
            </div>
          </div>

          {/* Change Hotkeys Modal - using shared component */}
          <KeyboardShortcutsModal
            open={isModalOpen}
            onOpenChange={setIsModalOpen}
          />
        </div>
      }
    />
  );
};

export default HotkeySetupStep;
