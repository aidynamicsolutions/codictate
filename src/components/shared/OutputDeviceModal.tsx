import React, { useEffect, useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { useSettings } from "@/hooks/useSettings";

const DeviceOption: React.FC<{
  name: string;
  isSelected: boolean;
  isSystemDefault?: boolean;
  onClick: () => void;
}> = ({ name, isSelected, isSystemDefault, onClick }) => {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full p-4 text-left rounded-lg border transition-all ${
        isSelected
          ? "border-primary bg-primary/5"
          : "border-border hover:border-primary/50 hover:bg-accent/50"
      }`}
    >
      <div className="flex flex-col gap-1">
        <span className="font-medium text-foreground">
          {name}
          {isSystemDefault && (
            <span className="ml-2 text-xs text-muted-foreground">
              ({t("common.default") || "Default"})
            </span>
          )}
        </span>
      </div>
    </button>
  );
};

interface OutputDeviceModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export const OutputDeviceModal: React.FC<OutputDeviceModalProps> = ({
  open,
  onOpenChange,
}) => {
  const { t } = useTranslation();
  const {
    outputDevices,
    refreshOutputDevices,
    getSetting,
    updateSetting,
    resetSetting,
  } = useSettings();

  // Poll for device list updates while dialog is open
  useEffect(() => {
    if (!open) return;

    refreshOutputDevices();
    const intervalId = setInterval(() => {
      refreshOutputDevices();
    }, 2000);

    return () => clearInterval(intervalId);
  }, [open, refreshOutputDevices]);

  // Derived state for selection
  const selectedDevice = getSetting("selected_output_device") || "Default";
  const actualDevices = useMemo(
    () => outputDevices.filter((d) => d.name !== "Default"),
    [outputDevices]
  );
  const systemDefaultDevice = useMemo(
    () => actualDevices.find((d) => d.is_default),
    [actualDevices]
  );

  const effectiveSelectedDevice = useMemo(() => {
    if ((selectedDevice === "Default" || selectedDevice === "default") && systemDefaultDevice) {
      return systemDefaultDevice.name;
    }
    return selectedDevice;
  }, [selectedDevice, systemDefaultDevice]);

  // Sorting
  const sortedDevices = useMemo(() => {
    return [...actualDevices].sort((a, b) => {
      const aIsSelected = a.name === effectiveSelectedDevice;
      const bIsSelected = b.name === effectiveSelectedDevice;
      if (aIsSelected && !bIsSelected) return -1;
      if (!aIsSelected && bIsSelected) return 1;
      if (a.is_default && !b.is_default) return -1;
      if (!a.is_default && b.is_default) return 1;
      return a.name.localeCompare(b.name);
    });
  }, [actualDevices, effectiveSelectedDevice]);

  const handleDeviceSelect = useCallback(
    async (device: { name: string; is_default: boolean }) => {
      if (device.is_default) {
           await resetSetting("selected_output_device");
      } else {
           await updateSetting("selected_output_device", device.name);
      }
      onOpenChange(false);
    },
    [updateSetting, resetSetting, onOpenChange]
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            {t("settings.sound.outputDevice.title")}
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-2 mt-4 max-h-64 overflow-y-auto">
          {sortedDevices.map((device) => {
            const isSelected = effectiveSelectedDevice === device.name;
            return (
              <DeviceOption
                key={device.index}
                name={device.name}
                isSelected={isSelected}
                isSystemDefault={device.is_default}
                onClick={() => handleDeviceSelect(device)}
              />
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
};
