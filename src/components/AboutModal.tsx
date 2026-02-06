import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { commands } from "@/bindings";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { Label } from "@/components/shared/ui/label";
import { Input } from "@/components/shared/ui/input";
import { Button } from "@/components/shared/ui/button";
import { ExternalLink, FolderOpen } from "lucide-react";

interface AboutModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export const AboutModal: React.FC<AboutModalProps> = ({ open, onOpenChange }) => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");
  const [appDirPath, setAppDirPath] = useState("");

  useEffect(() => {
    if (open) {
      getVersion().then(setVersion).catch(console.error);
      commands.getAppDirPath().then((result) => {
        if (result.status === "ok") {
          setAppDirPath(result.data);
        }
      }).catch(console.error);
    }
  }, [open]);

  const handleOpenAppData = async () => {
      try {
        await commands.openAppDataDir();
      } catch (error) {
        console.error("Failed to open app data directory:", error);
      }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px] p-0 overflow-hidden gap-0 bg-background border-border shadow-2xl rounded-2xl">
        <DialogHeader className="px-6 py-4 border-b border-border/40 bg-muted/20">
          <DialogTitle className="text-lg font-semibold tracking-tight">
            {t("settings.about.title")}
          </DialogTitle>
        </DialogHeader>

        <div className="p-6 space-y-6">
          {/* Version Info */}
          <div className="space-y-3">
             <div className="flex items-center justify-between">
                <Label className="text-sm font-medium text-foreground">
                  {t("settings.about.version.title")}
                </Label>
                <span className="text-sm font-mono text-muted-foreground bg-muted/40 px-2 py-1 rounded">
                  {t("settings.about.version.number", { version: version || "..." })}
                </span>
             </div>
             <p className="text-xs text-muted-foreground">
                {t("settings.about.version.description", { appName: t("appName") })}
             </p>
          </div>

          <div className="h-px bg-border/40" />

          {/* App Data Directory */}
          <div className="space-y-3">
             <Label className="text-sm font-medium text-foreground">
               {t("settings.about.appDataDirectory.title")}
             </Label>
             <div className="flex gap-2">
               <Input 
                 readOnly 
                 value={appDirPath} 
                 className="flex-1 font-mono text-xs h-9 bg-muted/20 text-muted-foreground focus-visible:ring-0 focus-visible:ring-offset-0"
                 tabIndex={-1}
               />
               <Button 
                 variant="outline" 
                 size="icon-sm" 
                 className="h-9 w-9 shrink-0"
                 onClick={handleOpenAppData}
                 title={t("settings.history.openFolder")}
               >
                 <FolderOpen className="h-4 w-4" />
               </Button>
             </div>
             <p className="text-xs text-muted-foreground">
               {t("settings.about.appDataDirectory.description", { appName: t("appName") })}
             </p>
          </div>

           <div className="h-px bg-border/40" />

          {/* Acknowledgments / Whisper */}
          <div className="space-y-3">
             <div className="flex items-center gap-2">
               <Label className="text-sm font-medium text-foreground">
                 {t("settings.about.acknowledgments.whisper.title")}
               </Label>
             </div>
             <p className="text-sm text-muted-foreground leading-relaxed">
               {t("settings.about.acknowledgments.whisper.details", { appName: t("appName") })}
             </p>
             <div className="flex items-center gap-4 pt-1">
                <a
                  href="https://github.com/openai/whisper"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-primary hover:underline flex items-center gap-1"
                >
                  {t("settings.about.acknowledgments.whisper.name")} <ExternalLink className="h-3 w-3" />
                </a>
                <a
                  href="https://github.com/ggerganov/whisper.cpp"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-primary hover:underline flex items-center gap-1"
                >
                  {t("settings.about.acknowledgments.whisperCpp.name")} <ExternalLink className="h-3 w-3" />
                </a>
             </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
};
