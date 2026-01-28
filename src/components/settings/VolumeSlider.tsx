import React from "react";
import { useTranslation } from "react-i18next";
import { Slider } from "@/components/shared/ui/slider";
import { Label } from "@/components/shared/ui/label";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { useSettings } from "../../hooks/useSettings";
import { logInfo } from "@/utils/logging";
import { InfoIcon } from "lucide-react";

export const VolumeSlider: React.FC<{ disabled?: boolean }> = ({
  disabled = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const audioFeedbackVolume = getSetting("audio_feedback_volume") ?? 0.5;

  const handleValueChange = (values: number[]) => {
      // Shadcn Slider returns array
      const value = values[0];
      updateSetting("audio_feedback_volume", value);
      // Log maybe on commit? but on change is fine for now, or use onValueCommit if available in shadcn wrapper?
      // Shadcn default slider uses onValueChange.
  };

  const handleValueCommit = (values: number[]) => {
      logInfo(`Volume changed to: ${values[0]}`, "fe");
  }

  return (
    <div className={`flex items-center justify-between py-4 ${disabled ? "opacity-50" : ""}`}>
        <div className="flex items-center gap-2">
            <Label className="text-sm font-medium">
                {t("settings.sound.volume.title")}
            </Label>
            <TooltipProvider>
                <Tooltip>
                    <TooltipTrigger asChild>
                    <InfoIcon className="h-4 w-4 text-muted-foreground cursor-help" />
                    </TooltipTrigger>
                    <TooltipContent>
                    <p className="max-w-xs">{t("settings.sound.volume.description")}</p>
                    </TooltipContent>
                </Tooltip>
            </TooltipProvider>
        </div>
        <div className="flex items-center gap-4 w-[280px]">
            <Slider
                value={[audioFeedbackVolume]}
                onValueChange={handleValueChange}
                onValueCommit={handleValueCommit}
                min={0}
                max={1}
                step={0.1}
                disabled={disabled}
                className="flex-1"
            />
            <span className="text-sm font-medium w-12 text-right">
                {Math.round(audioFeedbackVolume * 100)}%
            </span>
        </div>
    </div>
  );
};
