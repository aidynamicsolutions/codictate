import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { OutputDeviceModal } from "@/components/shared/OutputDeviceModal";
import { SettingsRow } from "../ui/SettingsRow";

interface OutputDeviceSelectorProps {
  disabled?: boolean;
}

export const OutputDeviceSelector: React.FC<OutputDeviceSelectorProps> =
  React.memo(({ disabled = false }) => {
    const { t } = useTranslation();
    const {
      getSetting,
      outputDevices,
      refreshOutputDevices,
      isLoading,
    } = useSettings();

    const [isModalOpen, setIsModalOpen] = useState(false);

    // Refresh devices on mount
    useEffect(() => {
        refreshOutputDevices();
    }, [refreshOutputDevices]);

    // Find the real system default device
    const systemDefaultDevice = outputDevices.find(
      (d) => d.is_default && d.name !== "Default" && d.name !== "default"
    );

    // Determine effective selection
    const selectedSetting = getSetting("selected_output_device");
    const isUsingSystemDefault = 
      selectedSetting === "default" || 
      selectedSetting === "Default" || 
      !selectedSetting;

    const effectiveSelectedDeviceName = isUsingSystemDefault 
        ? (systemDefaultDevice?.name || "Default") 
        : (selectedSetting || "Default");

    // "Default" label key
    const defaultLabel = t("common.default") || "Default";
    
    const displayLabel = isUsingSystemDefault
        ? `${effectiveSelectedDeviceName} (${defaultLabel})`
        : effectiveSelectedDeviceName;

    return (
      <>
        <SettingsRow
          title={t("settings.sound.outputDevice.title")}
          description={displayLabel}
          buttonLabel={t("common.change", "Change")}
          onButtonClick={() => setIsModalOpen(true)}
          disabled={disabled || isLoading}
        />

        <OutputDeviceModal 
            open={isModalOpen} 
            onOpenChange={setIsModalOpen}
        />
      </>
    );
  });
