import React from "react";
import { useTranslation } from "react-i18next";
import { Slider } from "@/components/shared/ui/slider";
import { SettingsRow } from "../ui/SettingsRow";
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
  };

  const handleValueCommit = (values: number[]) => {
      logInfo(`Volume changed to: ${values[0]}`, "fe");
  }

  const titleNode = (
    <div className="flex items-center gap-2">
      <span>{t("settings.sound.volume.title")}</span>
      <TooltipProvider>
          <Tooltip>
              <TooltipTrigger asChild>
              <InfoIcon className="h-3.5 w-3.5 text-muted-foreground/70 cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
              <p className="max-w-xs">{t("settings.sound.volume.description")}</p>
              </TooltipContent>
          </Tooltip>
      </TooltipProvider>
    </div>
  );

  return (
    <SettingsRow
        title={titleNode}
        disabled={disabled}
        className={disabled ? "opacity-50" : ""}
    >
        <div className="flex items-center gap-4 w-[200px] md:w-[240px]">
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
            <span className="text-sm font-medium w-12 text-right tabular-nums">
                {Math.round(audioFeedbackVolume * 100)}%
            </span>
        </div>
    </SettingsRow>
  );
};
