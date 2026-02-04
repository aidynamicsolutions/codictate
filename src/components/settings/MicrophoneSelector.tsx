import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { MicrophoneModal } from "@/components/shared/MicrophoneModal";
import { SettingsRow } from "../ui/SettingsRow";

export const MicrophoneSelector: React.FC = React.memo(
  () => {
    const { t } = useTranslation();
    const {
      getSetting,
      audioDevices,
      refreshAudioDevices,
      isLoading,
    } = useSettings();

    const [isModalOpen, setIsModalOpen] = useState(false);

    // Refresh devices on mount to ensure we have the system default name
    useEffect(() => {
        refreshAudioDevices();
    }, [refreshAudioDevices]);

    // Find the real system default device
    const systemDefaultMic = audioDevices.find(
      (d) => d.is_default && d.name !== "Default" && d.name !== "default"
    );
    
    // Determine effective selection
    const selectedSetting = getSetting("selected_microphone");
    const isUsingSystemDefault = 
      selectedSetting === "default" || 
      selectedSetting === "Default" || 
      !selectedSetting;

    const effectiveSelectedMicName = isUsingSystemDefault 
        ? (systemDefaultMic?.name || "Default") 
        : (selectedSetting || "Default");

    // "Default" label key
    const defaultLabel = t("common.default") || "Default";
    
    const displayLabel = isUsingSystemDefault
        ? `${effectiveSelectedMicName} (${defaultLabel})`
        : effectiveSelectedMicName;

    return (
      <>
        <SettingsRow
          title={t("settings.sound.microphone.title")}
          description={displayLabel}
          buttonLabel={t("common.change", "Change")}
          onButtonClick={() => setIsModalOpen(true)}
          disabled={isLoading}
        />

        <MicrophoneModal 
            open={isModalOpen} 
            onOpenChange={setIsModalOpen} 
            manageAudio={true}
        />
      </>
    );
  },
);
