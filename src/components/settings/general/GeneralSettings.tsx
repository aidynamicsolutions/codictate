import React, { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { ClamshellMicrophoneSelector } from "../ClamshellMicrophoneSelector";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingsRow } from "../../ui/SettingsRow";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { AudioFeedback } from "../AudioFeedback";
import { useSettings } from "../../../hooks/useSettings";

import { VolumeSlider } from "../VolumeSlider";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { KeyboardShortcutsModal } from "@/components/shared/KeyboardShortcutsModal";
import { LanguageSelectorModal } from "@/components/shared/LanguageSelectorModal";
import { getLanguageLabel } from "@/lib/constants/languageData";
import { Globe } from "lucide-react";
import { useEffect, useRef } from "react";
import { useUpdateStore } from "../../../stores/updateStore";
import { ShowOverlay } from "../ShowOverlay";
import { TranslateToEnglish } from "../TranslateToEnglish";
import { ModelUnloadTimeoutSetting } from "../ModelUnloadTimeout";
import { ShowUnloadModelInTray } from "../ShowUnloadModelInTray";

import { StartHidden } from "../StartHidden";
import { AutostartToggle } from "../AutostartToggle";
import { PasteMethodSetting } from "../PasteMethod";
import { PasteLastSmartInsertion } from "../PasteLastSmartInsertion";
import { TypingToolSetting } from "../TypingTool";
import { ClipboardHandlingSetting } from "../ClipboardHandling";
import { PostProcessingToggle } from "../PostProcessingToggle";
import { FillerWordFilter } from "../FillerWordFilter";
import { HallucinationFilter } from "../HallucinationFilter";
import { ResetAllSettings } from "../ResetAllSettings";
import { UpdateChecksToggle } from "../UpdateChecksToggle";
import { CheckForUpdates } from "../CheckForUpdates";
import { AppLanguageSelector } from "../AppLanguageSelector";

export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, audioFeedbackEnabled } = useSettings();

  
  const [isShortcutsModalOpen, setIsShortcutsModalOpen] = useState(false);
  const [isLanguageModalOpen, setIsLanguageModalOpen] = useState(false);
  const selectedLanguage = getSetting("selected_language") || "auto";

  // Helper to get language display text
  const getLanguageDisplay = () => {
    if (selectedLanguage === "auto") {
      return (
        <span className="flex items-center gap-2">
          <Globe className="h-3.5 w-3.5 text-muted-foreground" />
          {t("settings.general.language.auto")}
        </span>
      );
    }
    return getLanguageLabel(selectedLanguage) || selectedLanguage;
  };

  // Get OS type for determining default shortcut display
  const osType = useMemo(() => type(), []);

  // Get current push-to-talk binding for display
  const bindings = getSetting("bindings") || {};
  const transcribeBinding = bindings["transcribe"];
  const currentPttKey = transcribeBinding?.current_binding || (osType === "macos" ? "fn" : "ctrl+shift");

  // Format the binding for display (e.g., "fn" -> "fn", "ctrl+shift" -> "Ctrl+Shift")
  const formatBindingForDisplay = (binding: string): string => {
    if (!binding) return osType === "macos" ? "fn" : "Ctrl+Shift";
    
    // Keep "fn" lowercase, format others with proper capitalization
    if (binding.toLowerCase() === "fn") return "fn";
    
    return binding
      .split("+")
      .map((key) => {
        const trimmed = key.trim().toLowerCase();
        if (trimmed === "fn") return "fn";
        if (trimmed.length === 1) return trimmed;
        return trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
      })
      .join("+");
  };

  const appSettingsRef = useRef<HTMLDivElement>(null);
  const shouldScroll = useUpdateStore((state) => state.shouldScrollToUpdates);
  const setShouldScroll = useUpdateStore((state) => state.setShouldScrollToUpdates);

  useEffect(() => {
    if (shouldScroll && appSettingsRef.current) {
        appSettingsRef.current.scrollIntoView({ behavior: "smooth", block: "center" });
        setShouldScroll(false);
    }
  }, [shouldScroll, setShouldScroll]);

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6 py-4">
      <SettingsGroup title={t("settings.general.title")}>
        {/* Keyboard shortcuts row */}
        <SettingsRow
          title={t("settings.general.shortcut.title", { appName: t("appName") })}
          description={
            <>
              {t("settings.general.keyboardShortcuts.holdPrefix", "Hold")}{" "}
              <strong>{formatBindingForDisplay(currentPttKey)}</strong>{" "}
              {t("settings.general.keyboardShortcuts.holdSuffix", "and speak.")}
            </>
          }
          buttonLabel={t("common.change", "Change")}
          onButtonClick={() => setIsShortcutsModalOpen(true)}
        />

        {/* Microphone row */}
        <MicrophoneSelector />

        {/* Languages row */}
        <SettingsRow
            title={t("settings.general.language.title")}
            description={getLanguageDisplay()}
            buttonLabel={t("common.change", "Change")}
            onButtonClick={() => setIsLanguageModalOpen(true)}
        />
      </SettingsGroup>

      <SettingsGroup title={t("settings.sound.title")}>
        <MuteWhileRecording descriptionMode="tooltip" grouped={true} />
        {/* Clamshell mode microphone - only shown on laptops */}
        <ClamshellMicrophoneSelector descriptionMode="tooltip" grouped={true} />
        <AudioFeedback descriptionMode="tooltip" grouped={true} />
        <OutputDeviceSelector disabled={!audioFeedbackEnabled} />
        <VolumeSlider disabled={!audioFeedbackEnabled} />
      </SettingsGroup>
      
      <div ref={appSettingsRef}>
          <SettingsGroup title={t("settings.application.title", "Application")}>
            <CheckForUpdates />
            <UpdateChecksToggle descriptionMode="tooltip" grouped={true} />
            <AppLanguageSelector descriptionMode="tooltip" grouped={true} />
          </SettingsGroup>
      </div>

      <SettingsGroup title={t("settings.advanced.title")}>
        <StartHidden descriptionMode="tooltip" grouped={true} />
        <AutostartToggle descriptionMode="tooltip" grouped={true} />
        <ShowOverlay descriptionMode="tooltip" grouped={true} />
        <PasteMethodSetting descriptionMode="tooltip" grouped={true} />
        <PasteLastSmartInsertion descriptionMode="tooltip" grouped={true} />
        <TypingToolSetting descriptionMode="tooltip" grouped={true} />
        <ClipboardHandlingSetting descriptionMode="tooltip" grouped={true} />
        <TranslateToEnglish descriptionMode="tooltip" grouped={true} />
        <ModelUnloadTimeoutSetting descriptionMode="tooltip" grouped={true} />
        <ShowUnloadModelInTray descriptionMode="tooltip" grouped={true} />
        <FillerWordFilter descriptionMode="tooltip" grouped={true} />
        <HallucinationFilter descriptionMode="tooltip" grouped={true} />
        <PostProcessingToggle descriptionMode="tooltip" grouped={true} />

        <ResetAllSettings />
      </SettingsGroup>

      {/* Keyboard Shortcuts Modal */}
      <KeyboardShortcutsModal
        open={isShortcutsModalOpen}
        onOpenChange={setIsShortcutsModalOpen}
      />
      
      {/* Language Selection Modal */}
      <LanguageSelectorModal
        open={isLanguageModalOpen}
        onOpenChange={setIsLanguageModalOpen}
      />
    </div>
  );
};
