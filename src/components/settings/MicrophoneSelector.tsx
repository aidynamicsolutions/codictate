import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { ResetButton } from "../ui/ResetButton";
import { useSettings } from "../../hooks/useSettings";

interface MicrophoneSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const MicrophoneSelector: React.FC<MicrophoneSelectorProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
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

    // Ensure audio devices are loaded
    React.useEffect(() => {
      if (audioDevices.length === 0) {
        refreshAudioDevices();
      }
    }, [audioDevices.length, refreshAudioDevices]);

    // Find the real system default device (ignoring the "Default" placeholder added by the store)
    const systemDefaultMic = audioDevices.find((d) => d.is_default && d.name !== "Default" && d.name !== "default");
    
    // If setting is "Default" (or "default"), we conceptually select the system default device
    // But we still display it as its real name + (Default)
    const effectiveSelectedMic = 
      (getSetting("selected_microphone") === "default" || getSetting("selected_microphone") === "Default")
        ? systemDefaultMic?.name || "Default" 
        : getSetting("selected_microphone") || "Default";

    const handleMicrophoneSelect = async (deviceName: string) => {
      // If user selected the system default device, save "Default" 
      // OR save the actual name? User request: "remove default as an option".
      // Onboarding saves the specific name. So we save the specific name.
      await updateSetting("selected_microphone", deviceName);
    };

    const handleReset = async () => {
      await resetSetting("selected_microphone");
    };

    // Filter out "Default" from the list, relying on is_default flag to identify the default device
    // Fallback: If no system default is detected, we MUST show the "Default" option, otherwise the user sees "Select microphone..." with no valid option.
    const showDefaultOption = !systemDefaultMic;

    const microphoneOptions = audioDevices
      .filter((device) => {
        if (device.name === "Default" || device.name === "default") {
          return showDefaultOption;
        }
        return true;
      })
      .map((device) => {
        const isDefault = device.is_default;
        return {
          value: device.name,
          label: isDefault ? `${device.name} (${t("common.default")})` : device.name, // Assuming we have a translation for "default", otherwise hardcode or use existing key
        };
      });

    return (
      <SettingContainer
        title={t("settings.sound.microphone.title")}
        description={t("settings.sound.microphone.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <div className="flex items-center space-x-1">
          <Dropdown
            options={microphoneOptions}
            selectedValue={effectiveSelectedMic}
            onSelect={handleMicrophoneSelect}
            placeholder={
              isLoading || audioDevices.length === 0
                ? t("settings.sound.microphone.loading")
                : t("settings.sound.microphone.placeholder")
            }
            disabled={
              isUpdating("selected_microphone") ||
              isLoading ||
              audioDevices.length === 0
            }
            onRefresh={refreshAudioDevices}
          />
          <ResetButton
            onClick={handleReset}
            disabled={isUpdating("selected_microphone") || isLoading}
          />
        </div>
      </SettingContainer>
    );
  },
);
