import React, { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Pencil, InfoIcon } from "lucide-react";
import { formatKeyCombination, type OSType } from "../../lib/utils/keyboard";
import { useSettings } from "../../hooks/useSettings";
import { useShortcutRecorder } from "../../hooks/useShortcutRecorder";
import { commands } from "@/bindings";
import { toast } from "sonner";
import { type } from "@tauri-apps/plugin-os";
import { logError } from "@/utils/logging";
import { Label } from "@/components/shared/ui/label";
import { Button } from "@/components/shared/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";

interface CodictateShortcutProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  shortcutId: string;
  disabled?: boolean;
}

export const CodictateShortcut: React.FC<CodictateShortcutProps> = ({
  descriptionMode = "tooltip",
  shortcutId,
  disabled = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateBinding, isLoading } = useSettings();
  const containerRef = useRef<HTMLDivElement>(null);

  const bindings = getSetting("bindings") || {};
  const binding = bindings[shortcutId];

  // Get OS type for formatting
  const osType: OSType = (() => {
    const detected = type();
    if (detected === "macos") return "macos";
    if (detected === "windows") return "windows";
    if (detected === "linux") return "linux";
    return "unknown";
  })();

  const { isRecording, displayKeys, startRecording, error, warning } = useShortcutRecorder({
    onSave: async (shortcut) => {
      try {
        await updateBinding(shortcutId, shortcut);
      } catch (err) {
        logError(`Failed to change binding: ${err}`, "fe-shortcuts");
        toast.error(
          t("settings.general.shortcut.errors.set", {
            error: String(err),
          })
        );
        throw err;
      }
    },
    onCancel: () => {
      commands.resumeBinding(shortcutId).catch((err) =>
        logError(`Failed to resume binding: ${err}`, "fe-shortcuts")
      );
    },
    onRecordingStart: () => {
      commands.suspendBinding(shortcutId).catch((err) =>
        logError(`Failed to suspend binding: ${err}`, "fe-shortcuts")
      );
    },
    onRecordingEnd: () => {
    },
    requireModifier: true,
    containerRef,
    t,
  });

  if (isLoading) {
    return (
        <div className="flex items-center justify-between py-4">
             <div className="text-sm text-muted-foreground">{t("settings.general.shortcut.loading")}</div>
        </div>
    );
  }

  if (Object.keys(bindings).length === 0 || !binding) {
     return (
        <div className="flex items-center justify-between py-4">
             <div className="text-sm text-muted-foreground">{t("settings.general.shortcut.none")}</div>
        </div>
    );
  }

  // Get translated name and description for the binding
  const translatedName = t(
    `settings.general.shortcut.bindings.${shortcutId}.name`,
    binding.name
  );
  const translatedDescription = t(
    `settings.general.shortcut.bindings.${shortcutId}.description`,
    binding.description
  );

  const formatCurrentKeys = (): string => {
    if (displayKeys.length === 0) return t("settings.general.shortcut.pressKeys");
    return formatKeyCombination(displayKeys.join("+"), osType);
  };

  return (
    <div className={`flex items-center justify-between py-4 ${disabled ? "opacity-50 pointer-events-none" : ""}`}>
         <div className="flex items-center gap-2">
            <Label className="text-sm font-medium">
                {translatedName}
            </Label>
            {descriptionMode === "tooltip" && (
                <TooltipProvider>
                <Tooltip>
                    <TooltipTrigger asChild>
                    <InfoIcon className="h-4 w-4 text-muted-foreground cursor-help" />
                    </TooltipTrigger>
                    <TooltipContent>
                    <p className="max-w-xs">{translatedDescription}</p>
                    </TooltipContent>
                </Tooltip>
                </TooltipProvider>
            )}
        </div>
        <div className="flex flex-col items-end gap-1">
             <div className="flex items-center" ref={containerRef}>
                {isRecording ? (
                    <div className="px-3 py-2 text-sm font-semibold border border-primary bg-primary/10 rounded-md min-w-[120px] text-center">
                    {formatCurrentKeys()}
                    </div>
                ) : (
                    <Button
                        variant="outline"
                        size="sm"
                        className="font-mono"
                        onClick={startRecording}
                        disabled={disabled}
                    >
                    <span>{formatKeyCombination(binding.current_binding, osType)}</span>
                    <Pencil className="ml-2 h-3.5 w-3.5" />
                    </Button>
                )}
             </div>
             {error && <span className="text-xs text-destructive">{error}</span>}
             {warning && !error && <span className="text-xs text-yellow-600 dark:text-yellow-500">{warning}</span>}
        </div>
        {descriptionMode === "inline" && (
                <p className="text-sm text-muted-foreground mt-1 col-span-2">
                    {translatedDescription}
                </p>
        )}
    </div>
  );
};
