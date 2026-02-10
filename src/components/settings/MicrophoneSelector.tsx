import React, { useState, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { MicrophoneModal } from "@/components/shared/MicrophoneModal";
import { SettingsRow } from "../ui/SettingsRow";
import { isDefaultMicSetting, resolveDefaultMicName } from "@/utils/microphoneUtils";

export const MicrophoneSelector: React.FC = React.memo(
  () => {
    const { t } = useTranslation();
    const {
      settings,  // Subscribe to settings object directly for re-render on changes
      audioDevices,
      refreshAudioDevices,
      isLoading,
    } = useSettings();

    const [isModalOpen, setIsModalOpen] = useState(false);

    // Refresh devices on mount to ensure we have the device list
    useEffect(() => {
        refreshAudioDevices();
    }, [refreshAudioDevices]);
    
    // Determine effective selection - now reading directly from settings
    const selectedSetting = settings?.selected_microphone;
    const isUsingSystemDefault = isDefaultMicSetting(selectedSetting);

    // Resolve the actual mic name when Default is selected (display only — backend is authoritative)
    const resolvedDefaultName = useMemo(
      () => isUsingSystemDefault ? resolveDefaultMicName(audioDevices) : null,
      [isUsingSystemDefault, audioDevices],
    );

    const displayLabel = isUsingSystemDefault
        ? (resolvedDefaultName
            ? t("settings.sound.microphone.defaultWithName", { name: resolvedDefaultName })
            : (t("common.default") || "Default"))
        : (selectedSetting || "Default");

    const effectiveDevice = isUsingSystemDefault ? null : audioDevices.find(d => d.name === selectedSetting);
    const showBluetoothWarning = effectiveDevice?.is_bluetooth || false;

    return (
      <div className="flex flex-col gap-2">
        <SettingsRow
          title={t("settings.sound.microphone.title")}
          description={displayLabel}
          buttonLabel={t("common.change", "Change")}
          onButtonClick={() => setIsModalOpen(true)}
          disabled={isLoading}
        />

        {showBluetoothWarning && (
          <div className="text-amber-500 text-sm bg-amber-500/10 p-3 rounded-md border border-amber-500/20">
            {t("settings.sound.microphone.bluetoothWarning")}
          </div>
        )}
        
        {/* If we prevented an auto-switch (this is harder to detect purely on frontend without more state, 
            but we can infer it if "Default" is selected, the system default IS bluetooth, 
            but our effective device is NOT bluetooth/different) 
            For now, let's stick to the explicit warning.
        */}

        <MicrophoneModal 
            open={isModalOpen} 
            onOpenChange={setIsModalOpen} 
            manageAudio={true}
        />
      </div>
    );
  },
);
