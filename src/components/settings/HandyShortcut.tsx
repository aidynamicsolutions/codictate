import React, { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Pencil } from "lucide-react";
import { formatKeyCombination, type OSType } from "../../lib/utils/keyboard";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";
import { useShortcutRecorder } from "../../hooks/useShortcutRecorder";
import { commands } from "@/bindings";
import { toast } from "sonner";
import { type } from "@tauri-apps/plugin-os";
import { logError } from "@/utils/logging";

interface HandyShortcutProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  shortcutId: string;
  disabled?: boolean;
}

export const HandyShortcut: React.FC<HandyShortcutProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
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

  // Use the shared shortcut recorder hook
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
        throw err; // Re-throw to let hook handle it
      }
    },
    onCancel: () => {
      // Resume the suspended binding on cancel
      commands.resumeBinding(shortcutId).catch((err) =>
        logError(`Failed to resume binding: ${err}`, "fe-shortcuts")
      );
    },
    onRecordingStart: () => {
      // Suspend the binding while recording to avoid triggering actions
      commands.suspendBinding(shortcutId).catch((err) =>
        logError(`Failed to suspend binding: ${err}`, "fe-shortcuts")
      );
    },
    onRecordingEnd: () => {
      // Resume the binding after recording (success case)
      // Note: on cancel, onCancel is called which also resumes
    },
    requireModifier: true,
    containerRef,
  });

  // If still loading, show loading state
  if (isLoading) {
    return (
      <SettingContainer
        title={t("settings.general.shortcut.title")}
        description={t("settings.general.shortcut.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <div className="text-sm text-mid-gray">
          {t("settings.general.shortcut.loading")}
        </div>
      </SettingContainer>
    );
  }

  // If no bindings are loaded, show empty state
  if (Object.keys(bindings).length === 0) {
    return (
      <SettingContainer
        title={t("settings.general.shortcut.title")}
        description={t("settings.general.shortcut.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <div className="text-sm text-mid-gray">
          {t("settings.general.shortcut.none")}
        </div>
      </SettingContainer>
    );
  }

  if (!binding) {
    return (
      <SettingContainer
        title={t("settings.general.shortcut.title")}
        description={t("settings.general.shortcut.notFound")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <div className="text-sm text-mid-gray">
          {t("settings.general.shortcut.none")}
        </div>
      </SettingContainer>
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

  // Format display keys for recording state
  const formatCurrentKeys = (): string => {
    if (displayKeys.length === 0) return t("settings.general.shortcut.pressKeys");
    return formatKeyCombination(displayKeys.join("+"), osType);
  };

  return (
    <SettingContainer
      title={translatedName}
      description={translatedDescription}
      descriptionMode={descriptionMode}
      grouped={grouped}
      disabled={disabled}
      layout="horizontal"
    >
      <div className="flex flex-col items-end gap-1">
        <div className="flex items-center" ref={containerRef}>
          {isRecording ? (
            <div className="px-3 py-1.5 text-sm font-semibold border border-logo-primary bg-logo-primary/30 rounded min-w-[120px] text-center">
              {formatCurrentKeys()}
            </div>
          ) : (
            <button
              type="button"
              className="flex items-center gap-2 px-3 py-1.5 text-sm font-semibold bg-muted/50 border border-border hover:bg-muted rounded cursor-pointer hover:border-primary/50 transition-colors"
              onClick={startRecording}
            >
              <span>{formatKeyCombination(binding.current_binding, osType)}</span>
              <Pencil className="h-3.5 w-3.5 text-muted-foreground" />
            </button>
          )}
        </div>
        {error && <span className="text-xs text-destructive">{error}</span>}
        {warning && !error && <span className="text-xs text-yellow-600 dark:text-yellow-500">{warning}</span>}
      </div>
    </SettingContainer>
  );
};
