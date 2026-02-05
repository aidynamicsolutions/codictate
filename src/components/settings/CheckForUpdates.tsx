import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsRow } from "../ui/SettingsRow";
import { useUpdateCheck } from "@/hooks/useUpdateCheck";
import { ProgressBar } from "../shared";
import { Button } from "../shared/ui/button";
import { Loader2 } from "lucide-react";

export const CheckForUpdates: React.FC = () => {
  const { t } = useTranslation();
  const {
    isChecking,
    updateAvailable,
    isInstalling,
    downloadProgress,
    showUpToDate,
    isPendingRestart,
    checkForUpdates,
    installUpdate,
    restartApp,
    downloadSpeed,
    downloadEta,
  } = useUpdateCheck();

  const getStatusText = () => {
    if (isPendingRestart) return t("settings.application.updates.restartRequired", "Restart to update");
    if (isInstalling) {
      return downloadProgress > 0 && downloadProgress < 100
        ? t("footer.downloading", {
            progress: downloadProgress.toString().padStart(3),
          })
        : downloadProgress === 100
        ? t("footer.installing")
        : t("footer.preparing");
    }
    if (isChecking) return t("footer.checkingUpdates");
    if (showUpToDate) return t("footer.upToDate");
    if (updateAvailable) return t("footer.updateAvailableShort");
    return t("settings.application.updates.description", "Current version: v{{version}}", { version: "0.7.1" }); // Ideally version should be dynamic
  };

  const getButtonLabel = () => {
    if (isPendingRestart) return t("settings.application.updates.restart", "Restart");
    if (isInstalling) return t("footer.installing"); // Or disabled
    if (updateAvailable) return t("settings.application.updates.install", "Install Update");
    return t("settings.application.updates.check", "Check for Updates...");
  };

  const handleAction = () => {
      if (isPendingRestart) {
          restartApp();
      } else if (updateAvailable) {
          installUpdate();
      } else {
          checkForUpdates(true);
      }
  };
  
  const isActionDisabled = isChecking || isInstalling;

  // We need to get the app version. 
  // In `Footer.tsx` it used `getVersion()`.
  // I should probably fetch it here or move it to a hook. 
  // For now I'll use a simple state for version or just show the status.
  
  const [version, setVersion] = React.useState("");

  React.useEffect(() => {
    import("@tauri-apps/api/app").then(async (app) => {
        try {
            const v = await app.getVersion();
            setVersion(v);
        } catch (e) {
            console.error(e);
        }
    });
  }, []);

  return (
    <div className="flex flex-col w-full">
        <SettingsRow
            title={t("settings.application.updates.title", "Update application")}
            description={
                <div className="flex flex-col gap-2">
                    <div className="flex items-center gap-2">
                        <span>{version ? `v${version}` : "..."}</span>
                        {(isChecking || showUpToDate || updateAvailable || isInstalling || isPendingRestart) && (
                             <span className={(updateAvailable || isPendingRestart) ? "text-logo-primary font-medium" : "text-muted-foreground"}>
                                {
                                    // Reuse footer translations for status if appropriate 
                                    // but we might want slightly different wording for settings page.
                                    // For now reusing footer strings as they are quite standard.
                                    isChecking ? (
                                        <span className="flex items-center gap-2">
                                            <Loader2 className="w-3 h-3 animate-spin" />
                                            {t("footer.checkingUpdates")}
                                        </span>
                                    ) :
                                    isPendingRestart ? t("settings.application.updates.restartRequired", "Restart to update") :
                                    showUpToDate ? t("footer.upToDate") :
                                    updateAvailable ? t("footer.updateAvailableShort") :
                                    isInstalling ? t("footer.installing") : ""
                                }
                             </span>
                        )}
                    </div>
                     {isInstalling && downloadProgress >= 0 && downloadProgress < 100 && (
                        <div className="w-full max-w-[300px] mt-1">
                            <ProgressBar
                                progress={[
                                    {
                                    id: "update",
                                    percentage: downloadProgress,
                                    },
                                ]}
                                size="small"
                            />
                            <div className="flex justify-between mt-1 px-1">
                                {downloadSpeed > 0 && (
                                    <span className="text-xs text-muted-foreground tabular-nums">
                                        {downloadSpeed.toFixed(1)} MB/s
                                    </span>
                                )}
                                {downloadEta !== null && downloadEta > 0 && (
                                    <span className="text-xs text-muted-foreground tabular-nums">
                                        {downloadEta < 60 
                                            ? `${downloadEta}s` 
                                            : `${Math.floor(downloadEta / 60)}m ${downloadEta % 60}s`} remaining
                                    </span>
                                )}
                            </div>
                        </div>
                    )}
                </div>
            }
            buttonLabel={getButtonLabel()}
            onButtonClick={handleAction}
            disabled={isActionDisabled}
        />
    </div>
  );
};
