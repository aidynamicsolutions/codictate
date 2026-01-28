import React from "react";
import { useTranslation } from "react-i18next";
import { DeviceSelector } from "./DeviceSelector";
import { useSettings } from "../../hooks/useSettings";
import { logInfo } from "@/utils/logging";

interface MicrophoneSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean; // Kept for compatibility
}

export const MicrophoneSelector: React.FC<MicrophoneSelectorProps> = React.memo(
  ({ descriptionMode = "tooltip" }) => {
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

    // Find the real system default device
    const systemDefaultMic = audioDevices.find((d) => d.is_default && d.name !== "Default" && d.name !== "default");
    
    // If setting is "Default", we conceptually select the system default device
    const effectiveSelectedMic = 
      (getSetting("selected_microphone") === "default" || getSetting("selected_microphone") === "Default")
        ? systemDefaultMic?.name || "Default" 
        : getSetting("selected_microphone") || "Default";

    const handleMicrophoneSelect = async (deviceName: string) => {
      logInfo(`Microphone selected: ${deviceName}`, "fe");
      await updateSetting("selected_microphone", deviceName);
    };

    const handleReset = async () => {
      logInfo("Microphone setting reset", "fe");
      await resetSetting("selected_microphone");
    };

    const handleRefresh = async () => {
        logInfo("Refreshing audio devices", "fe");
        await refreshAudioDevices();
    };

    // Filter out "Default" from the list
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
          label: isDefault ? `${device.name} (${t("common.default")})` : device.name,
        };
      });

    return (
      <DeviceSelector
        label={t("settings.sound.microphone.title")}
        description={t("settings.sound.microphone.description")}
        value={effectiveSelectedMic}
        options={microphoneOptions}
        onSelect={handleMicrophoneSelect}
        onRefresh={handleRefresh}
        onReset={handleReset}
        isLoading={isLoading}
        isUpdating={isUpdating("selected_microphone")}
        placeholder={t("settings.sound.microphone.placeholder")}
        loadingLabel={t("settings.sound.microphone.loading")}
        refreshLabel={t("common.refresh")}
        resetLabel={t("common.reset")}
        descriptionMode={descriptionMode}
      />
    );
  },
);
