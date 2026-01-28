import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { logInfo } from "@/utils/logging";

interface MuteWhileRecordingToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean; // Kept for compatibility
}

export const MuteWhileRecording: React.FC<MuteWhileRecordingToggleProps> =
  React.memo(({ descriptionMode = "tooltip" }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const muteEnabled = getSetting("mute_while_recording") ?? false;

    const handleCheckedChange = (checked: boolean) => {
      logInfo(`Mute while recording changed to: ${checked}`, "fe");
      updateSetting("mute_while_recording", checked);
    };

    return (
      <ToggleSwitch
        checked={muteEnabled}
        onChange={handleCheckedChange}
        disabled={isUpdating("mute_while_recording")}
        label={t("settings.debug.muteWhileRecording.label")}
        description={t("settings.debug.muteWhileRecording.description")}
        descriptionMode={descriptionMode}
        id="mute-while-recording"
      />
    );
  });
