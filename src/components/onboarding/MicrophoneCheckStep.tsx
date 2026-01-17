import React, { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import OnboardingLayout from "./OnboardingLayout";
import { useSettings } from "@/hooks/useSettings";
import { commands } from "@/bindings";

interface MicrophoneCheckStepProps {
  onContinue: () => void;
  onBack: () => void;
}

/**
 * Automatic Gain Control (AGC) for audio visualization
 * Industry best practice: track recent peak levels and normalize display relative to that
 * This ensures the meter shows visible movement for any input level
 */
class AudioAGC {
  private peakLevel = 0.1; // Start with a small baseline
  private readonly attackTime = 0.3; // Fast attack - quickly adapt to louder sounds
  private readonly releaseTime = 2.0; // Slow release - gradually reduce gain when quiet
  private readonly minPeak = 0.05; // Minimum peak to prevent division by very small numbers
  private readonly maxPeak = 1.0; // Maximum peak
  private lastUpdateTime = Date.now();

  /**
   * Process incoming levels and return normalized values (0-1 range, visually meaningful)
   */
  process(levels: number[]): number[] {
    const now = Date.now();
    const deltaTime = (now - this.lastUpdateTime) / 1000;
    this.lastUpdateTime = now;

    // Find current max level
    const currentMax = Math.max(...levels, 0.001);

    // Update peak with attack/release dynamics
    if (currentMax > this.peakLevel) {
      // Attack: quickly rise to new peak
      const attackRate = 1 - Math.exp(-deltaTime / this.attackTime);
      this.peakLevel += (currentMax - this.peakLevel) * attackRate;
    } else {
      // Release: slowly decay peak
      const releaseRate = 1 - Math.exp(-deltaTime / this.releaseTime);
      this.peakLevel -= (this.peakLevel - currentMax) * releaseRate * 0.5;
    }

    // Clamp peak to valid range
    this.peakLevel = Math.max(this.minPeak, Math.min(this.maxPeak, this.peakLevel));

    // Normalize levels relative to current peak (AGC effect)
    // This makes the bars show significant movement even for quiet speech
    const normalizedLevels = levels.map((level) => {
      const normalized = level / this.peakLevel;
      // Apply slight curve for more pleasing visual (emphasize mid-range)
      return Math.pow(Math.min(1, normalized), 0.8);
    });

    return normalizedLevels;
  }

  reset() {
    this.peakLevel = 0.1;
    this.lastUpdateTime = Date.now();
  }
}

// Audio level bars component with AGC normalization built-in
const AudioLevelBars: React.FC<{ levels: number[] }> = ({ levels }) => {
  return (
    <div className="flex items-end justify-center gap-1.5 h-10">
      {levels.map((v, i) => (
        <div
          key={i}
          className="w-2 rounded-sm bg-primary transition-all duration-75"
          style={{
            // Height scales from 6px (silent) to 40px (loud)
            height: `${Math.max(6, Math.min(40, 6 + v * 34))}px`,
            opacity: Math.max(0.4, Math.min(1, 0.4 + v * 0.6)),
          }}
        />
      ))}
    </div>
  );
};

// Microphone option component for the dialog
const MicrophoneOption: React.FC<{
  name: string;
  isSelected: boolean;
  isSystemDefault?: boolean;
  levels?: number[];
  onClick: () => void;
}> = ({ name, isSelected, isSystemDefault, levels, onClick }) => {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full p-4 text-left rounded-lg border transition-all ${
        isSelected
          ? "border-primary bg-primary/5"
          : "border-border hover:border-primary/50 hover:bg-accent/50"
      }`}
    >
      <div className="flex items-center justify-between">
        <div className="flex flex-col gap-1">
          <span className="font-medium text-foreground">
            {name}
            {isSystemDefault && (
              <span className="ml-2 text-xs text-muted-foreground">
                ({t("onboarding.microphoneCheck.dialog.default")})
              </span>
            )}
          </span>
        </div>
        {isSelected && levels && (
          <div className="flex items-end gap-0.5 h-6">
            {levels.slice(0, 8).map((v, i) => (
              <div
                key={i}
                className="w-1 rounded-sm bg-primary transition-all duration-75"
                style={{
                  height: `${Math.max(4, Math.min(24, 4 + v * 20))}px`,
                  opacity: Math.max(0.4, Math.min(1, 0.4 + v * 0.6)),
                }}
              />
            ))}
          </div>
        )}
      </div>
    </button>
  );
};

export const MicrophoneCheckStep: React.FC<MicrophoneCheckStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();
  const {
    audioDevices,
    refreshAudioDevices,
    getSetting,
    updateSetting,
    isLoading,
  } = useSettings();

  // Audio level state - these are AGC-normalized for display
  const [displayLevels, setDisplayLevels] = useState<number[]>(Array(16).fill(0));
  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  const agcRef = useRef(new AudioAGC());



  // Dialog state
  const [isDialogOpen, setIsDialogOpen] = useState(false);

  // Get current microphone setting - "Default" or a specific device name
  const selectedMicrophone = getSetting("selected_microphone") || "Default";

  // Filter out the "Default" entry - only show actual microphone devices
  const actualMicrophones = useMemo(
    () => audioDevices.filter((d) => d.name !== "Default"),
    [audioDevices]
  );

  // Find the system default microphone (has is_default=true among actual devices)
  const systemDefaultMic = useMemo(
    () => actualMicrophones.find((d) => d.is_default),
    [actualMicrophones]
  );

  // Determine which microphone should appear selected:
  // - If selectedMicrophone is "Default", highlight the system default mic
  // - Otherwise, highlight the specifically selected mic
  const effectiveSelectedMic = useMemo(() => {
    if (selectedMicrophone === "Default" && systemDefaultMic) {
      return systemDefaultMic.name;
    }
    return selectedMicrophone;
  }, [selectedMicrophone, systemDefaultMic]);

  // Sort microphones: selected mic at top, then system default, then alphabetically
  const sortedMicrophones = useMemo(() => {
    return [...actualMicrophones].sort((a, b) => {
      // Currently selected mic comes first
      const aIsSelected = a.name === effectiveSelectedMic;
      const bIsSelected = b.name === effectiveSelectedMic;
      if (aIsSelected && !bIsSelected) return -1;
      if (!aIsSelected && bIsSelected) return 1;

      // Then system default
      if (a.is_default && !b.is_default) return -1;
      if (!a.is_default && b.is_default) return 1;

      // Then sort alphabetically
      return a.name.localeCompare(b.name);
    });
  }, [actualMicrophones, effectiveSelectedMic]);

  // Poll for device list updates while dialog is open (to catch newly connected devices like AirPods)
  useEffect(() => {
    if (!isDialogOpen) return;

    // Refresh immediately when dialog opens
    refreshAudioDevices();

    // Poll every 2 seconds while dialog is open
    const intervalId = setInterval(() => {
      refreshAudioDevices();
    }, 2000);

    return () => clearInterval(intervalId);
  }, [isDialogOpen, refreshAudioDevices]);

  // Start mic preview on mount
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

    // Listen for mic level updates
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

  // Handle microphone change
  const handleMicrophoneSelect = useCallback(
    async (deviceName: string) => {
      await updateSetting("selected_microphone", deviceName);
      // Reset AGC for new device
      agcRef.current.reset();
      // Restart mic preview with new device
      await commands.stopMicPreview();
      await commands.startMicPreview();
      setIsDialogOpen(false);
    },
    [updateSetting]
  );



  // Cleanup is handled by useEffect when component unmounts
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
              <div className="flex items-center justify-center gap-3">

                <Button
                  variant="outline"
                  onClick={() => setIsDialogOpen(true)}
                  disabled={isLoading}
                >
                  {t("onboarding.microphoneCheck.changeMicrophone")}
                </Button>

                <Button onClick={handleContinue}>
                  {t("onboarding.microphoneCheck.yes")}
                </Button>
              </div>
            </div>
          </div>

          {/* Microphone Selection Dialog */}
          <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
            <DialogContent className="max-w-md">
              <DialogHeader>
                <DialogTitle>
                  {t("onboarding.microphoneCheck.dialog.title")}
                </DialogTitle>
              </DialogHeader>

              <div className="flex flex-col gap-2 mt-4">
                {/* Microphone list - selected mic at top */}
                <div className="flex flex-col gap-2 max-h-64 overflow-y-auto">
                  {sortedMicrophones.map((device) => {
                    const isSelected = effectiveSelectedMic === device.name;

                    return (
                      <MicrophoneOption
                        key={device.index}
                        name={device.name}
                        isSelected={isSelected}
                        isSystemDefault={device.is_default}
                        levels={isSelected ? displayLevels : undefined}
                        onClick={() => handleMicrophoneSelect(device.name)}
                      />
                    );
                  })}
                </div>
              </div>
            </DialogContent>
          </Dialog>
        </div>
      }
    />
  );
};

export default MicrophoneCheckStep;
