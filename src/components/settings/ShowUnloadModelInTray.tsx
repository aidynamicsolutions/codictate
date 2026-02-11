import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface ShowUnloadModelInTrayProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const ShowUnloadModelInTray: React.FC<ShowUnloadModelInTrayProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { settings, updateSetting, isUpdating } = useSettings();

  const enabled = settings?.show_unload_model_in_tray ?? false;

  return (
    <ToggleSwitch
      checked={enabled}
      onChange={(value) => updateSetting("show_unload_model_in_tray", value)}
      isUpdating={isUpdating("show_unload_model_in_tray")}
      label={t("settings.advanced.showUnloadModelInTray.label")}
      description={t("settings.advanced.showUnloadModelInTray.description")}
      descriptionMode={descriptionMode}
      className={grouped ? "border-t-0" : ""}
    />
  );
};
