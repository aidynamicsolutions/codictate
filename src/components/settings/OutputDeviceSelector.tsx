import React from "react";
import { useTranslation } from "react-i18next";
import { DeviceSelector } from "./DeviceSelector";
import { useSettings } from "../../hooks/useSettings";
import { logInfo } from "@/utils/logging";

interface OutputDeviceSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  disabled?: boolean;
  grouped?: boolean; // Kept for compatibility
}

export const OutputDeviceSelector: React.FC<OutputDeviceSelectorProps> =
  React.memo(
    ({ descriptionMode = "tooltip", disabled = false }) => {
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

      // Find the real system default device
      const systemDefaultDevice = outputDevices.find((d) => d.is_default && d.name !== "Default" && d.name !== "default");

      // If setting is "Default", we conceptually select the system default device
      const effectiveSelectedDevice = 
        (getSetting("selected_output_device") === "default" || getSetting("selected_output_device") === "Default")
          ? systemDefaultDevice?.name || "Default" 
          : getSetting("selected_output_device") || "Default";

      const handleOutputDeviceSelect = async (deviceName: string) => {
        logInfo(`Output device selected: ${deviceName}`, "fe");
        await updateSetting("selected_output_device", deviceName);
      };

      const handleReset = async () => {
        logInfo("Output device setting reset", "fe");
        await resetSetting("selected_output_device");
      };

      const handleRefresh = async () => {
         logInfo("Refreshing output devices", "fe");
         await refreshOutputDevices();
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
        .map((device) => {
          const isDefault = device.is_default;
          return {
            value: device.name,
            label: isDefault ? `${device.name} (${t("common.default")})` : device.name,
          };
        });

      return (
        <DeviceSelector
          label={t("settings.sound.outputDevice.title")}
          description={t("settings.sound.outputDevice.description")}
          value={effectiveSelectedDevice}
          options={outputDeviceOptions}
          onSelect={handleOutputDeviceSelect}
          onRefresh={handleRefresh}
          onReset={handleReset}
          isLoading={isLoading}
          isUpdating={isUpdating("selected_output_device")}
          disabled={disabled || outputDevices.length === 0}
          placeholder={isLoading ? t("settings.sound.outputDevice.loading") : t("settings.sound.outputDevice.placeholder")}
          loadingLabel={t("settings.sound.outputDevice.loading")}
          refreshLabel={t("common.refresh")}
          resetLabel={t("common.reset")}
          descriptionMode={descriptionMode}
        />
      );
    },
  );
