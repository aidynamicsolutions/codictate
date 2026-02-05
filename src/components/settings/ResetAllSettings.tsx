import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettingsStore } from "@/stores/settingsStore";
import { SettingsRow } from "../ui/SettingsRow";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/shared/ui/alert-dialog";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import { RotateCcw } from "lucide-react";

export const ResetAllSettings: React.FC = () => {
  const { t } = useTranslation();
  const resetAllSettings = useSettingsStore((state) => state.resetAllSettings);
  const [isOpen, setIsOpen] = useState(false);
  const [isResetting, setIsResetting] = useState(false);

  const handleReset = async () => {
    setIsResetting(true);
    try {
      await resetAllSettings();
      toast.success(t("settings.advanced.resetSuccess", "Settings reset to defaults"));
      setIsOpen(false);
    } catch (error) {
      console.error("Failed to reset settings:", error);
      toast.error(t("settings.advanced.resetError", "Failed to reset settings"));
    } finally {
      setIsResetting(false);
    }
  };

  return (
    <>
      <SettingsRow
        title={t("settings.advanced.reset.title", "Reset All Settings")}
        description={t(
          "settings.advanced.reset.description",
          "Restore all settings to their default values. This will not delete your recordings."
        )}
        buttonLabel={t("settings.advanced.reset.button", "Reset to Defaults")}
        onButtonClick={() => setIsOpen(true)}
        buttonVariant="destructive"
      />

      <AlertDialog open={isOpen} onOpenChange={setIsOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("settings.advanced.reset.confirmationTitle", "Are you sure?")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                "settings.advanced.reset.confirmationDescription",
                "This action cannot be undone. This will reset all your settings to their default values. Your recordings and history will remain safe."
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isResetting}>
              {t("common.cancel", "Cancel")}
            </AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={(e: React.MouseEvent) => {
                e.preventDefault();
                handleReset();
              }}
              disabled={isResetting}
            >
              {isResetting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t("common.resetting", "Resetting...")}
                </>
              ) : (
                t("common.confirmReset", "Reset All Settings")
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
};
