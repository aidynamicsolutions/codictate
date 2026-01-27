import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { ResetButton } from "../ui/ResetButton";
import { useSettings } from "../../hooks/useSettings";
import type { AudioDevice } from "@/bindings";

interface OutputDeviceSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  disabled?: boolean;
}

export const OutputDeviceSelector: React.FC<OutputDeviceSelectorProps> =
  React.memo(
    ({ descriptionMode = "tooltip", grouped = false, disabled = false }) => {
      const { t } = useTranslation();
      const {
        getSetting,
        updateSetting,
        resetSetting,
        isUpdating,
        isLoading,
        outputDevices,
        refreshOutputDevices,
      } = useSettings();

      // Ensure output devices are loaded
      React.useEffect(() => {
        if (outputDevices.length === 0) {
          refreshOutputDevices();
        }
      }, [outputDevices.length, refreshOutputDevices]);

      // Find the real system default device (ignoring the "Default" placeholder added by the store)
      const systemDefaultDevice = outputDevices.find((d) => d.is_default && d.name !== "Default" && d.name !== "default");

      // If setting is "Default" (or "default"), we conceptually select the system default device
      const effectiveSelectedDevice = 
        (getSetting("selected_output_device") === "default" || getSetting("selected_output_device") === "Default")
          ? systemDefaultDevice?.name || "Default" 
          : getSetting("selected_output_device") || "Default";

      const handleOutputDeviceSelect = async (deviceName: string) => {
        await updateSetting("selected_output_device", deviceName);
      };

      const handleReset = async () => {
        await resetSetting("selected_output_device");
      };

      // Fallback: If no system default is detected, we MUST show the "Default" option.
      const showDefaultOption = !systemDefaultDevice;

      const outputDeviceOptions = outputDevices
        .filter((device) => {
            if (device.name === "Default" || device.name === "default") {
                return showDefaultOption;
            }
            return true;
        })
        .map((device: AudioDevice) => {
          const isDefault = device.is_default;
          return {
            value: device.name,
            label: isDefault ? `${device.name} (${t("common.default")})` : device.name,
          };
        });

      return (
        <SettingContainer
          title={t("settings.sound.outputDevice.title")}
          description={t("settings.sound.outputDevice.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
          disabled={disabled}
        >
          <div className="flex items-center space-x-1">
            <Dropdown
              options={outputDeviceOptions}
              selectedValue={effectiveSelectedDevice}
              onSelect={handleOutputDeviceSelect}
              placeholder={
                isLoading || outputDevices.length === 0
                  ? t("settings.sound.outputDevice.loading")
                  : t("settings.sound.outputDevice.placeholder")
              }
              disabled={
                disabled ||
                isUpdating("selected_output_device") ||
                isLoading ||
                outputDevices.length === 0
              }
              onRefresh={refreshOutputDevices}
            />
            <ResetButton
              onClick={handleReset}
              disabled={
                disabled || isUpdating("selected_output_device") || isLoading
              }
            />
          </div>
        </SettingContainer>
      );
    },
  );
