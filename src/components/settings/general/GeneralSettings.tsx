import React, { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingsRow } from "../../ui/SettingsRow";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { AudioFeedback } from "../AudioFeedback";
import { useSettings } from "../../../hooks/useSettings";
import { useModelStore } from "../../../stores/modelStore";
import { VolumeSlider } from "../VolumeSlider";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { KeyboardShortcutsModal } from "@/components/shared/KeyboardShortcutsModal";

export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, audioFeedbackEnabled } = useSettings();
  const { currentModel, getModelInfo } = useModelStore();
  const currentModelInfo = getModelInfo(currentModel);
  const showLanguageSelector = currentModelInfo?.engine_type === "Whisper";

  const [isShortcutsModalOpen, setIsShortcutsModalOpen] = useState(false);

  // Get OS type for determining default shortcut display
  const osType = useMemo(() => type(), []);

  // Get current push-to-talk binding for display
  const bindings = getSetting("bindings") || {};
  const transcribeBinding = bindings["transcribe"];
  const currentPttKey = transcribeBinding?.current_binding || (osType === "macos" ? "fn" : "ctrl+shift");

  // Format the binding for display (e.g., "fn" -> "fn", "ctrl+shift" -> "Ctrl+Shift")
  const formatBindingForDisplay = (binding: string): string => {
    if (!binding) return osType === "macos" ? "fn" : "Ctrl+Shift";
    
    // Keep "fn" lowercase, format others with proper capitalization
    if (binding.toLowerCase() === "fn") return "fn";
    
    return binding
      .split("+")
      .map((key) => {
        const trimmed = key.trim().toLowerCase();
        if (trimmed === "fn") return "fn";
        if (trimmed.length === 1) return trimmed;
        return trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
      })
      .join("+");
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.general.title")}>
        {/* Keyboard shortcuts row */}
        <SettingsRow
          title={t("settings.general.shortcut.title", { appName: t("appName") })}
          description={
            <>
              {t("settings.general.keyboardShortcuts.holdPrefix", "Hold")}{" "}
              <strong>{formatBindingForDisplay(currentPttKey)}</strong>{" "}
              {t("settings.general.keyboardShortcuts.holdSuffix", "and speak.")}
            </>
          }
          buttonLabel={t("common.change", "Change")}
          onButtonClick={() => setIsShortcutsModalOpen(true)}
        />

        {/* Microphone row */}
        <MicrophoneSelector descriptionMode="tooltip" grouped={true} />

        {/* Languages placeholder row */}
        {showLanguageSelector && (
          <SettingsRow
            title={t("settings.general.language.title")}
            description={t("settings.general.language.comingSoon", "Language selection coming soon")}
            buttonLabel={t("common.change", "Change")}
            onButtonClick={() => {}}
            disabled={true}
          />
        )}
      </SettingsGroup>

      <SettingsGroup title={t("settings.sound.title")}>
        <MuteWhileRecording descriptionMode="tooltip" grouped={true} />
        <AudioFeedback descriptionMode="tooltip" grouped={true} />
        <OutputDeviceSelector
          descriptionMode="tooltip"
          grouped={true}
          disabled={!audioFeedbackEnabled}
        />
        <VolumeSlider disabled={!audioFeedbackEnabled} />
      </SettingsGroup>

      {/* Keyboard Shortcuts Modal */}
      <KeyboardShortcutsModal
        open={isShortcutsModalOpen}
        onOpenChange={setIsShortcutsModalOpen}
      />
    </div>
  );
};
