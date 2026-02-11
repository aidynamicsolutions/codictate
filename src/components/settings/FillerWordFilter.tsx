import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface FillerWordFilterProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const FillerWordFilter: React.FC<FillerWordFilterProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { settings, updateSetting, isUpdating } = useSettings();

  const enabled = settings?.enable_filler_word_filter ?? true;

  return (
    <ToggleSwitch
      checked={enabled}
      onChange={(value) => updateSetting("enable_filler_word_filter", value)}
      isUpdating={isUpdating("enable_filler_word_filter")}
      label={t("settings.debug.fillerWordFilter.label")}
      description={t("settings.debug.fillerWordFilter.description")}
      descriptionMode={descriptionMode}
      className={grouped ? "border-t-0" : ""}
    />
  );
};
