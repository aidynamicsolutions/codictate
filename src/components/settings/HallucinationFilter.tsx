import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface HallucinationFilterProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const HallucinationFilter: React.FC<HallucinationFilterProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { settings, updateSetting, isUpdating } = useSettings();

  const enabled = settings?.enable_hallucination_filter ?? true;

  return (
    <ToggleSwitch
      checked={enabled}
      onChange={(value) => updateSetting("enable_hallucination_filter", value)}
      isUpdating={isUpdating("enable_hallucination_filter")}
      label={t("settings.debug.hallucinationFilter.label")}
      description={t("settings.debug.hallucinationFilter.description")}
      descriptionMode={descriptionMode}
      className={grouped ? "border-t-0" : ""}
    />
  );
};
