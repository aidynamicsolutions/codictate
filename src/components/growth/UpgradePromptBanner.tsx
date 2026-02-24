import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";

interface UpgradePromptBannerProps {
  visible: boolean;
  onUpgrade: () => void;
  onDismiss: () => void;
}

export function UpgradePromptBanner({
  visible,
  onUpgrade,
  onDismiss,
}: UpgradePromptBannerProps): JSX.Element | null {
  const { t } = useTranslation();

  if (!visible) {
    return null;
  }

  return (
    <div className="w-full max-w-5xl px-4 pt-4">
      <div className="rounded-xl border border-border/70 bg-card/80 px-4 py-3 shadow-sm">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-1">
            <p className="text-sm font-semibold text-foreground">
              {t("growth.upgradePrompt.title")}
            </p>
            <p className="text-sm text-muted-foreground">
              {t("growth.upgradePrompt.description")}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <Button variant="outline" size="sm" onClick={onDismiss}>
              {t("growth.upgradePrompt.dismiss")}
            </Button>
            <Button size="sm" onClick={onUpgrade}>
              {t("growth.upgradePrompt.cta")}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
