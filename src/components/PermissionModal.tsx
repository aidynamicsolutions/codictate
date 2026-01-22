import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { Button } from "@/components/shared/ui/button";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Mic, Keyboard } from "lucide-react";

export type PermissionType = "accessibility" | "microphone";

interface PermissionModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  type: PermissionType;
}

const SETTINGS_URLS: Record<PermissionType, string> = {
  accessibility:
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
  microphone:
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
};

const ICONS: Record<PermissionType, React.ElementType> = {
  accessibility: Keyboard,
  microphone: Mic,
};

export default function PermissionModal({
  open,
  onOpenChange,
  type,
}: PermissionModalProps) {
  const { t } = useTranslation();
  const Icon = ICONS[type];

  const handleOpenSettings = async () => {
    await openUrl(SETTINGS_URLS[type]);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={() => {}}>
      <DialogContent
        showCloseButton={false}
        className="max-w-sm text-center"
        onInteractOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.preventDefault()}
      >
        <DialogHeader className="items-center">
          <div className="w-12 h-12 rounded-full bg-destructive/10 flex items-center justify-center mb-2">
            <Icon className="w-6 h-6 text-destructive" />
          </div>
          <DialogTitle className="text-xl font-semibold">
            {t(`permissions.modal.${type}.title`)}
          </DialogTitle>
          <DialogDescription className="text-center text-muted-foreground">
            {t(`permissions.modal.${type}.description`)}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter className="flex-col gap-2 sm:flex-col">
          <Button onClick={handleOpenSettings} className="w-full">
            {t(`permissions.modal.${type}.openSettings`)}
          </Button>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            className="w-full"
          >
            {t("permissions.modal.dismiss")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
