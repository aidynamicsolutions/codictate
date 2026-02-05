
import React from "react";
import { useTranslation } from "react-i18next";
import { logInfo } from "@/utils/logging";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface UpdateChecksToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const UpdateChecksToggle: React.FC<UpdateChecksToggleProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("update_checks_enabled") ?? false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(val) => {
          logInfo(`Update checks toggled: ${val}`, "fe");
          updateSetting("update_checks_enabled", val);
        }}
        isUpdating={isUpdating("update_checks_enabled")}
        label={t("settings.application.autoUpdate.label", "Automatically check for updates")}
        description={t("settings.application.autoUpdate.description", "Automatically check for updates on startup.")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);
