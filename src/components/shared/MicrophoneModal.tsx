import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { useSettings } from "@/hooks/useSettings";
import { commands } from "@/bindings";
import { AudioAGC } from "@/utils/audioAGC";

// Audio level bars component with AGC normalization built-in
export const AudioLevelBars: React.FC<{ levels: number[] }> = ({ levels }) => {
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

interface MicrophoneModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  manageAudio?: boolean; // Whether the modal should start/stop preview
}

export const MicrophoneModal: React.FC<MicrophoneModalProps> = ({
  open,
  onOpenChange,
  manageAudio = true,
}) => {
  const { t } = useTranslation();
  const {
    audioDevices,
    refreshAudioDevices,
    getSetting,
    updateSetting,
  } = useSettings();

  // Audio level state - these are AGC-normalized for display
  const [displayLevels, setDisplayLevels] = useState<number[]>(Array(16).fill(0));
  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  const agcRef = useRef(new AudioAGC());

  // Poll for device list updates while dialog is open
  useEffect(() => {
    if (!open) return;

    refreshAudioDevices();
    const intervalId = setInterval(() => {
      refreshAudioDevices();
    }, 2000);

    return () => clearInterval(intervalId);
  }, [open, refreshAudioDevices]);

  // Handle audio preview management
  useEffect(() => {
    if (!open) return;

    let unlistenFn: (() => void) | undefined;

    const startAudio = async () => {
      try {
        if (manageAudio) {
           agcRef.current.reset();
           await commands.startMicPreview();
        }
        
        // Always listen if open, even if not managing start/stop
        // This assumes if manageAudio=false, someone else started it.
        unlistenFn = await listen<number[]>("mic-level", (event) => {
            const rawLevels = event.payload;
            const smoothed = smoothedLevelsRef.current.map((prev, i) => {
              const target = rawLevels[i] || 0;
              return prev * 0.6 + target * 0.4;
            });
            smoothedLevelsRef.current = smoothed;
            const normalized = agcRef.current.process(smoothed.slice(0, 16));
            setDisplayLevels(normalized);
        });

      } catch (error) {
        console.error("Error starting mic preview/listener:", error);
      }
    };

    startAudio();

    return () => {
      if (unlistenFn) unlistenFn();
      if (manageAudio) {
        commands.stopMicPreview().catch(console.error);
      }
    };
  }, [open, manageAudio]);

  // Derived state for selection
  const selectedMicrophone = getSetting("selected_microphone") || "Default";
  const actualMicrophones = useMemo(
    () => audioDevices.filter((d) => d.name !== "Default"),
    [audioDevices]
  );
  const systemDefaultMic = useMemo(
    () => actualMicrophones.find((d) => d.is_default),
    [actualMicrophones]
  );

  const effectiveSelectedMic = useMemo(() => {
    if ((selectedMicrophone === "Default" || selectedMicrophone === "default") && systemDefaultMic) {
      return systemDefaultMic.name;
    }
    return selectedMicrophone;
  }, [selectedMicrophone, systemDefaultMic]);

  // Sorting
  const sortedMicrophones = useMemo(() => {
    return [...actualMicrophones].sort((a, b) => {
      const aIsSelected = a.name === effectiveSelectedMic;
      const bIsSelected = b.name === effectiveSelectedMic;
      if (aIsSelected && !bIsSelected) return -1;
      if (!aIsSelected && bIsSelected) return 1;
      if (a.is_default && !b.is_default) return -1;
      if (!a.is_default && b.is_default) return 1;
      return a.name.localeCompare(b.name);
    });
  }, [actualMicrophones, effectiveSelectedMic]);

  const handleMicrophoneSelect = useCallback(
    async (device: { name: string; is_default: boolean }) => {
      // Always save the explicit device name, even if it's the system default.
      // This is critical because if we reset to "Default", the backend applies
      // Bluetooth-avoidance logic (preferring built-in mic). By saving the 
      // explicit name, the user's intent is preserved and Bluetooth devices work.
      await updateSetting("selected_microphone", device.name);
      
      // Reset AGC and restart preview to apply new device
      agcRef.current.reset();
      await commands.stopMicPreview();
      await commands.startMicPreview();
      
      onOpenChange(false);
    },
    [updateSetting, onOpenChange]
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            {t("onboarding.microphoneCheck.dialog.title")}
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-2 mt-4">
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
                  onClick={() => handleMicrophoneSelect(device)}
                />
              );
            })}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
};
