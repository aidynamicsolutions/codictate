import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { logInfo } from "@/utils/logging";

interface PushToTalkProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean; // Kept for compatibility
}

export const PushToTalk: React.FC<PushToTalkProps> = React.memo(
  ({ descriptionMode = "tooltip" }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const pttEnabled = getSetting("push_to_talk") || false;

    const handleCheckedChange = (checked: boolean) => {
      logInfo(`Push to talk changed to: ${checked}`, "fe");
      updateSetting("push_to_talk", checked);
    };

    return (
      <ToggleSwitch
        checked={pttEnabled}
        onChange={handleCheckedChange}
        disabled={isUpdating("push_to_talk")}
        label={t("settings.general.pushToTalk.label")}
        description={t("settings.general.pushToTalk.description")}
        descriptionMode={descriptionMode}
        id="push-to-talk"
      />
    );
  },
);
