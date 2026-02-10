import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { useSettings } from "@/hooks/useSettings";
import { commands } from "@/bindings";
import { AudioAGC } from "@/utils/audioAGC";
import { isDefaultMicSetting } from "@/utils/microphoneUtils";

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
  subtitle?: string;
  isSelected: boolean;
  isBluetooth?: boolean;
  bluetoothBadgeLabel?: string;
  bluetoothTooltipText?: string;
  levels?: number[];
  onClick: () => void;
}> = ({ name, subtitle, isSelected, isBluetooth, bluetoothBadgeLabel, bluetoothTooltipText, levels, onClick }) => {
  const button = (
    <button
      type="button"
      onClick={onClick}
      className={`w-full p-4 text-left rounded-lg border transition-all ${
        isSelected
          ? "border-primary bg-primary/5 dark:bg-primary/10"
          : "border-border/50 dark:border-border hover:border-primary/50 hover:bg-accent/50 dark:hover:bg-muted/50"
      }`}
    >
      <div className="flex items-center justify-between">
        <div className="flex flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <span className="font-medium text-foreground">{name}</span>
            {isBluetooth && bluetoothBadgeLabel && (
              <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/20">
                {bluetoothBadgeLabel}
              </span>
            )}
          </div>
          {subtitle && (
            <span className="text-xs text-muted-foreground">{subtitle}</span>
          )}
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

  if (isBluetooth && bluetoothTooltipText) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          {button}
        </TooltipTrigger>
        <TooltipContent side="bottom" className="max-w-xs text-center">
          {bluetoothTooltipText}
        </TooltipContent>
      </Tooltip>
    );
  }

  return button;
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
  const isUsingDefault = isDefaultMicSetting(selectedMicrophone);
  const actualMicrophones = useMemo(
    () => audioDevices.filter((d) => d.name !== "Default"),
    [audioDevices]
  );

  const effectiveSelectedMic = isUsingDefault ? "Default" : selectedMicrophone;

  // Sorting — bluetooth always at bottom, then selected device first, then alphabetical
  const sortedMicrophones = useMemo(() => {
    return [...actualMicrophones].sort((a, b) => {
      // Bluetooth devices always go to the bottom
      if (a.is_bluetooth && !b.is_bluetooth) return 1;
      if (!a.is_bluetooth && b.is_bluetooth) return -1;
      // Within same category, selected first
      const aIsSelected = !isUsingDefault && a.name === effectiveSelectedMic;
      const bIsSelected = !isUsingDefault && b.name === effectiveSelectedMic;
      if (aIsSelected && !bIsSelected) return -1;
      if (!aIsSelected && bIsSelected) return 1;
      return a.name.localeCompare(b.name);
    });
  }, [actualMicrophones, effectiveSelectedMic, isUsingDefault]);

  const handleSelectDefault = useCallback(async () => {
    // Reset to "default" — backend applies BT-avoidance, preferring built-in mic
    await updateSetting("selected_microphone", "default");
    agcRef.current.reset();
    await commands.stopMicPreview();
    await commands.startMicPreview();
    onOpenChange(false);
  }, [updateSetting, onOpenChange]);

  const handleMicrophoneSelect = useCallback(
    async (device: { name: string; is_default: boolean }) => {
      // Save explicit device name — preserves user intent (e.g., BT devices work)
      await updateSetting("selected_microphone", device.name);
      agcRef.current.reset();
      await commands.stopMicPreview();
      await commands.startMicPreview();
      onOpenChange(false);
    },
    [updateSetting, onOpenChange]
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md border-border/60 shadow-2xl dark:border-border dark:shadow-black/50 dark:bg-card">
        <DialogHeader>
          <DialogTitle>
            {t("onboarding.microphoneCheck.dialog.title")}
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-2 mt-4">
          <div className="flex flex-col gap-2 max-h-64 overflow-y-auto">
            {/* "Default" option — uses smart device selection with BT avoidance */}
            <MicrophoneOption
              name={t("onboarding.microphoneCheck.dialog.defaultOption")}
              subtitle={t("onboarding.microphoneCheck.dialog.defaultDescription")}
              isSelected={isUsingDefault}
              levels={isUsingDefault ? displayLevels : undefined}
              onClick={handleSelectDefault}
            />
            {sortedMicrophones.map((device) => {
              const isSelected = !isUsingDefault && effectiveSelectedMic === device.name;
              return (
                <MicrophoneOption
                  key={device.index}
                  name={device.name}
                  isSelected={isSelected}
                  isBluetooth={device.is_bluetooth}
                  bluetoothBadgeLabel={device.is_bluetooth ? t("settings.sound.microphone.bluetoothBadge") : undefined}
                  bluetoothTooltipText={device.is_bluetooth ? t("settings.sound.microphone.bluetoothTooltip") : undefined}
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
