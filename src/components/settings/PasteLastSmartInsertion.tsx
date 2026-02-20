import React from "react";
import { useTranslation } from "react-i18next";
import { logInfo } from "@/utils/logging";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { isPasteLastSmartInsertionEnabled } from "./pasteLastSmartInsertionUtils";

interface PasteLastSmartInsertionProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const PasteLastSmartInsertion: React.FC<PasteLastSmartInsertionProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = isPasteLastSmartInsertionEnabled(
      getSetting("paste_last_use_smart_insertion"),
    );

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(nextValue) => {
          logInfo(`Paste last smart insertion toggled: ${nextValue}`, "fe");
          updateSetting("paste_last_use_smart_insertion", nextValue);
        }}
        isUpdating={isUpdating("paste_last_use_smart_insertion")}
        label={t("settings.advanced.pasteLastSmartInsertion.label")}
        description={t("settings.advanced.pasteLastSmartInsertion.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
