import React from "react";
import { useTranslation } from "react-i18next";
import { InfoIcon, RotateCcw } from "lucide-react";

import { Button } from "@/components/shared/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/shared/ui/select";
import { Label } from "@/components/shared/ui/label";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { useSettings } from "../../hooks/useSettings";
import { LANGUAGES } from "../../lib/constants/languages";
import { logInfo } from "@/utils/logging";

interface LanguageSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean; // Kept for compatibility but ignored in new design
}

export const LanguageSelector: React.FC<LanguageSelectorProps> = ({
  descriptionMode = "tooltip",
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, resetSetting, isUpdating } = useSettings();

  const selectedLanguage = getSetting("selected_language") || "auto";

  const handleSelect = async (currentValue: string) => {
    logInfo(`Language selected: ${currentValue}`, "fe");
    await updateSetting("selected_language", currentValue);
  };

  const handleReset = async () => {
    logInfo("Language setting reset", "fe");
    await resetSetting("selected_language");
  };

  return (
    <div className="flex items-center justify-between py-4">
      <div className="flex items-center gap-2">
        <Label className="text-sm font-medium">
          {t("settings.general.language.title")}
        </Label>
        {descriptionMode === "tooltip" && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <InfoIcon className="h-4 w-4 text-muted-foreground cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="max-w-xs">
                  {t("settings.general.language.description")}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Select
          value={selectedLanguage}
          onValueChange={handleSelect}
          disabled={isUpdating("selected_language")}
        >
          <SelectTrigger className="w-[280px]">
            <SelectValue placeholder={t("settings.general.language.auto")} />
          </SelectTrigger>
          <SelectContent position="popper" className="max-h-[300px]">
             {/* Explicitly add Auto option if not in the list (though it is) */}
             <SelectItem value="auto">{t("settings.general.language.auto")}</SelectItem>
            {LANGUAGES.filter(l => l.value !== "auto").map((language) => (
              <SelectItem key={language.value} value={language.value}>
                {language.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleReset}
          disabled={isUpdating("selected_language")}
          title={t("common.reset") ?? "Reset"}
        >
          <RotateCcw className="h-4 w-4" />
          <span className="sr-only">{t("common.reset")}</span>
        </Button>
      </div>
      {descriptionMode === "inline" && (
        <p className="text-sm text-muted-foreground mt-1 col-span-2">
          {t("settings.general.language.description")}
        </p>
      )}
    </div>
  );
};
