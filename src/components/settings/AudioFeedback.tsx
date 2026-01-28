import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { logInfo } from "@/utils/logging";

interface AudioFeedbackProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean; // Kept for compatibility
}

export const AudioFeedback: React.FC<AudioFeedbackProps> = React.memo(
  ({ descriptionMode = "tooltip" }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const audioFeedbackEnabled = getSetting("audio_feedback") || false;

    const handleCheckedChange = (checked: boolean) => {
      logInfo(`Audio feedback changed to: ${checked}`, "fe");
      updateSetting("audio_feedback", checked);
    };

    return (
      <ToggleSwitch
        checked={audioFeedbackEnabled}
        onChange={handleCheckedChange}
        disabled={isUpdating("audio_feedback")}
        label={t("settings.sound.audioFeedback.label")}
        description={t("settings.sound.audioFeedback.description")}
        descriptionMode={descriptionMode}
        id="audio-feedback"
      />
    );
  },
);
