import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { Dropdown } from "../ui/Dropdown";
import { SettingsRow } from "../ui/SettingsRow";
import { Button } from "@/components/shared/ui/button";
import { useSettings } from "../../hooks/useSettings";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { InfoIcon, RotateCcw } from "lucide-react";

interface ClamshellMicrophoneSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const ClamshellMicrophoneSelector: React.FC<ClamshellMicrophoneSelectorProps> =
  React.memo(({ descriptionMode = "tooltip" }) => {
    const { t } = useTranslation();
    const {
      getSetting,
      updateSetting,
      resetSetting,
      isUpdating,
      isLoading,
      audioDevices,
      refreshAudioDevices,
    } = useSettings();

    const [isLaptop, setIsLaptop] = useState<boolean>(false);

    useEffect(() => {
      const checkIsLaptop = async () => {
        try {
          const result = await commands.isLaptop();
          if (result.status === "ok") {
            setIsLaptop(result.data);
          } else {
            setIsLaptop(false);
          }
        } catch (error) {
          console.error("Failed to check if device is laptop:", error);
          setIsLaptop(false);
        }
      };

      checkIsLaptop();
    }, []);

    // Only render on laptops
    if (!isLaptop) {
      return null;
    }

    const selectedClamshellMicrophone =
      getSetting("clamshell_microphone") === "default"
        ? "Default"
        : getSetting("clamshell_microphone") || "Default";

    const handleClamshellMicrophoneSelect = async (deviceName: string) => {
      await updateSetting("clamshell_microphone", deviceName);
    };

    const handleReset = async () => {
      await resetSetting("clamshell_microphone");
    };

    const microphoneOptions = audioDevices.map((device) => ({
      value: device.name,
      label: device.name,
    }));

    const titleNode = (
      <div className="flex items-center gap-2">
        <span>{t("settings.sound.clamshellMicrophone.title")}</span>
        {descriptionMode === "tooltip" && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <InfoIcon className="h-3.5 w-3.5 text-muted-foreground/70 cursor-help" />
              </TooltipTrigger>
              <TooltipContent>
                <p className="max-w-xs">{t("settings.sound.clamshellMicrophone.description")}</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>
    );

    return (
      <SettingsRow
        title={titleNode}
        description={descriptionMode === "inline" ? t("settings.sound.clamshellMicrophone.description") : undefined}
      >
        <div className="flex items-center space-x-1">
          <Dropdown
            options={microphoneOptions}
            selectedValue={selectedClamshellMicrophone}
            onSelect={handleClamshellMicrophoneSelect}
            placeholder={
              isLoading || audioDevices.length === 0
                ? t("common.loading")
                : t("settings.sound.microphone.placeholder")
            }
            disabled={
              isUpdating("clamshell_microphone") ||
              isLoading ||
              audioDevices.length === 0
            }
            onRefresh={refreshAudioDevices}
          />
          <Button
            variant="ghost"
            size="icon"
            onClick={handleReset}
            disabled={isUpdating("clamshell_microphone") || isLoading}
            title={t("common.reset")}
            className="h-8 w-8"
          >
            <RotateCcw className="h-4 w-4" />
          </Button>
        </div>
      </SettingsRow>
    );
  });

ClamshellMicrophoneSelector.displayName = "ClamshellMicrophoneSelector";
