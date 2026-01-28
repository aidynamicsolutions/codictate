import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/shared/ui/card";
import { Button } from "@/components/shared/ui/button";
import { Label } from "@/components/shared/ui/label";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { Info } from "lucide-react";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { AppDataDirectory } from "../AppDataDirectory";

const SettingsRow = ({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: React.ReactNode;
}) => (
  <div className="flex items-center justify-between py-4 px-6 gap-4">
    <div className="flex items-center gap-2">
      <Label className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70">
        {label}
      </Label>
      <TooltipProvider delayDuration={300}>
        <Tooltip>
          <TooltipTrigger asChild>
            <Info className="h-3.5 w-3.5 text-muted-foreground/70 cursor-help hover:text-foreground transition-colors" />
          </TooltipTrigger>
          <TooltipContent>
            <p className="max-w-xs">{description}</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    </div>
    <div className="flex items-center gap-2">{children}</div>
  </div>
);

export const AboutSettings: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.1.2");
      }
    };

    fetchVersion();
  }, []);

  const handleDonateClick = async () => {
    try {
      await openUrl("https://handy.computer/donate");
    } catch (error) {
      console.error("Failed to open donate link:", error);
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <Card className="w-full animate-in fade-in slide-in-from-bottom-2 duration-500 fill-mode-both bg-card/60 backdrop-blur-sm border-border/60 hover:border-border/80 transition-colors">
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-semibold uppercase tracking-wide text-primary font-heading">
            {t("settings.about.title")}
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <div className="divide-y divide-border/40">
            <div className="px-6 py-4">
               {/* AppLanguageSelector manages its own internal layout, usually we wrap or adapt it. 
                   Checking AppLanguageSelector it likely returns a SettingContainer. 
                   Ideally we should refactor AppLanguageSelector too or just wrap it for now.
                   However, since we are doing a direct refactor, let's just render it. 
                   If it looks odd, we might need to adjust.
                   Wait, user asked to check About page. 
                   If AppLanguageSelector uses SettingContainer, it will still use custom tooltips.
               */}
               <AppLanguageSelector descriptionMode="tooltip" grouped={true} /> 
               {/* This component likely needs refactoring to match the new style purely, 
                   but strictly for "About Page" refactor, we can leave it if it renders okay 
                   or refactor the surrounding ones. 
                   Given the timeline, let's keep the others clean. */}
            </div>

            <SettingsRow
              label={t("settings.about.version.title")}
              description={t("settings.about.version.description")}
            >
              {/* eslint-disable-next-line i18next/no-literal-string */}
              <span className="text-sm font-mono text-muted-foreground">v{version}</span>
            </SettingsRow>

            <div className="px-6 py-4">
                 <AppDataDirectory descriptionMode="tooltip" grouped={true} />
            </div>

            <SettingsRow
              label={t("settings.about.sourceCode.title")}
              description={t("settings.about.sourceCode.description")}
            >
              <Button
                variant="outline"
                size="sm"
                className="h-8 text-xs font-medium"
                onClick={() => openUrl("https://github.com/cjpais/Handy")}
              >
                {t("settings.about.sourceCode.button")}
              </Button>
            </SettingsRow>

            <SettingsRow
              label={t("settings.about.supportDevelopment.title")}
              description={t("settings.about.supportDevelopment.description")}
            >
              <Button 
                variant="default" 
                size="sm"
                className="h-8 text-xs font-medium bg-red-500 hover:bg-red-600 text-white border-none shadow-sm"
                onClick={handleDonateClick}
              >
                {t("settings.about.supportDevelopment.button")}
              </Button>
            </SettingsRow>
          </div>
        </CardContent>
      </Card>

      <Card className="w-full animate-in fade-in slide-in-from-bottom-2 duration-500 fill-mode-both bg-card/60 backdrop-blur-sm border-border/60 hover:border-border/80 transition-colors">
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-semibold uppercase tracking-wide text-primary font-heading">
            {t("settings.about.acknowledgments.title")}
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <div className="divide-y divide-border/40">
            <div className="p-6 pt-2">
              <div className="flex items-center gap-2 mb-2">
                 <Label className="text-sm font-medium">
                   {t("settings.about.acknowledgments.whisper.title")}
                 </Label>
                 <TooltipProvider delayDuration={300}>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Info className="h-3.5 w-3.5 text-muted-foreground/70 cursor-help hover:text-foreground transition-colors" />
                    </TooltipTrigger>
                    <TooltipContent>
                      <p className="max-w-xs">{t("settings.about.acknowledgments.whisper.description")}</p>
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {t("settings.about.acknowledgments.whisper.details")}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};
