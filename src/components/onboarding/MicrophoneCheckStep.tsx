import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/shared/ui/button";
import OnboardingLayout from "./OnboardingLayout";
import { useSettings } from "@/hooks/useSettings";
import { commands } from "@/bindings";
import { AudioAGC } from "@/utils/audioAGC";
import { MicrophoneModal, AudioLevelBars } from "@/components/shared/MicrophoneModal";

interface MicrophoneCheckStepProps {
  onContinue: () => void;
  onBack: () => void;
}

export const MicrophoneCheckStep: React.FC<MicrophoneCheckStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();
  const {
    refreshAudioDevices,
    isLoading,
  } = useSettings();

  // Audio level state - these are AGC-normalized for display
  const [displayLevels, setDisplayLevels] = useState<number[]>(Array(16).fill(0));
  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  const agcRef = useRef(new AudioAGC());

  // Dialog state
  const [isDialogOpen, setIsDialogOpen] = useState(false);

  // Start mic preview on mount & Listen for mic level updates
  useEffect(() => {
    const startMicPreview = async () => {
      try {
        // Reset AGC when starting
        agcRef.current.reset();
        
        const result = await commands.startMicPreview();
        if (result.status === "ok") {
          // Mic preview started successfully
        } else {
          console.error("Failed to start mic preview:", result.error);
        }
      } catch (error) {
        console.error("Error starting mic preview:", error);
      }
    };

    startMicPreview();
    refreshAudioDevices();

    const setupLevelListener = async () => {
      const unlisten = await listen<number[]>("mic-level", (event) => {
        const rawLevels = event.payload;

        // Apply smoothing to reduce jitter
        const smoothed = smoothedLevelsRef.current.map((prev, i) => {
          const target = rawLevels[i] || 0;
          return prev * 0.6 + target * 0.4; // Slightly less smoothing for more responsiveness
        });
        smoothedLevelsRef.current = smoothed;

        // Apply AGC normalization for display
        const normalized = agcRef.current.process(smoothed.slice(0, 16));
        setDisplayLevels(normalized);
      });

      return unlisten;
    };

    const unlistenPromise = setupLevelListener();

    return () => {
      // Stop mic preview on unmount
      commands.stopMicPreview();
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [refreshAudioDevices]);

  const handleContinue = () => onContinue();
  const handleBack = () => onBack();

  return (
    <OnboardingLayout
      currentStep="microphoneCheck"
      leftContent={
        <div className="flex flex-col h-full">
          {/* Back button - positioned at top */}
          <button
            type="button"
            onClick={handleBack}
            className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors w-fit mb-auto"
          >
            <ArrowLeft className="h-4 w-4" />
            {t("onboarding.microphoneCheck.back")}
          </button>

          {/* Content centered vertically */}
          <div className="flex flex-col gap-6 my-auto">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl max-w-[380px]">
              {t("onboarding.microphoneCheck.title")}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.microphoneCheck.subtitle")}
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
                {t("onboarding.microphoneCheck.question")}
              </p>

              {/* Audio level visualization */}
              <div className="bg-accent/30 rounded-lg p-4">
                <AudioLevelBars levels={displayLevels} />
              </div>

              {/* Action buttons */}
              <div className="flex items-center justify-end gap-3">

                <Button
                  variant="outline"
                  onClick={() => setIsDialogOpen(true)}
                  disabled={isLoading}
                >
                  {t("onboarding.microphoneCheck.changeMicrophone")}
                </Button>

                <Button onClick={handleContinue} className="min-w-[80px]">
                  {t("onboarding.microphoneCheck.yes")}
                </Button>
              </div>
            </div>
          </div>

          <MicrophoneModal 
            open={isDialogOpen} 
            onOpenChange={setIsDialogOpen}
            manageAudio={false} 
          />
        </div>
      }
    />
  );
};

export default MicrophoneCheckStep;
