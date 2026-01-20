import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";

interface SkipConfirmationModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Confirmation modal shown when user tries to skip critical onboarding steps
 * (like Permissions) without completing them.
 * 
 * Styled to match the reference Superwhisper modal design.
 */
export const SkipConfirmationModal: React.FC<SkipConfirmationModalProps> = ({
  open,
  onOpenChange,
  onConfirm,
  onCancel,
}) => {
  const { t } = useTranslation();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent showCloseButton={false} className="max-w-sm text-center">
        <DialogHeader className="items-center">
          {/* App Icon */}
          <div className="mb-2 flex h-16 w-16 items-center justify-center rounded-xl bg-gradient-to-br from-gray-700 to-gray-900 shadow-lg">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              className="h-8 w-8 text-white"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M12 18.75a6 6 0 0 0 6-6v-1.5m-6 7.5a6 6 0 0 1-6-6v-1.5m6 7.5v3.75m-3.75 0h7.5M12 15.75a3 3 0 0 1-3-3V4.5a3 3 0 1 1 6 0v8.25a3 3 0 0 1-3 3Z"
              />
            </svg>
          </div>
          <DialogTitle className="text-xl font-semibold">
            {t("onboarding.permissions.skipModal.title")}
          </DialogTitle>
          <DialogDescription className="text-center text-muted-foreground">
            {t("onboarding.permissions.skipModal.description", {
              appName: t("appName"),
            })}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter className="flex-col gap-2 sm:flex-col">
          <Button onClick={onConfirm} size="lg" className="w-full">
            {t("onboarding.permissions.skipModal.continueAnyway")}
          </Button>
          <Button
            onClick={onCancel}
            variant="outline"
            size="lg"
            className="w-full"
          >
            {t("onboarding.permissions.skipModal.goBack")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default SkipConfirmationModal;
