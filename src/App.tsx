import { useEffect, useRef, useState } from "react";
import { Toaster, toast } from "sonner";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getLanguageDirection, initializeRTL } from "@/lib/utils/rtl";
import "./App.css";
import AccessibilityPermissions from "./components/AccessibilityPermissions";
import MicrophonePermissions from "./components/MicrophonePermissions";
import Onboarding from "./components/onboarding";
import {
  SidebarProvider,
  SidebarInset,
  SidebarTrigger,
} from "@/components/shared/ui/sidebar";
import { Sidebar, SidebarSection, SECTIONS_CONFIG } from "./components/Sidebar";
import { useSettings } from "./hooks/useSettings";
import { useTauriEvent } from "./hooks/useTauriEvent";
import { commands } from "@/bindings";
import { initLogging, logError, logInfo } from "@/utils/logging";
import { trackUiAnalyticsEvent } from "@/utils/analytics";
import { useModelStore } from "./stores/modelStore";
import { useSettingsStore } from "./stores/settingsStore";
import { useUpdateStore } from "./stores/updateStore";
import { AboutModal } from "./components/AboutModal";
import { UpgradePromptBanner } from "./components/growth/UpgradePromptBanner";

const renderSettingsContent = (
  section: SidebarSection,
  onNavigate: (section: SidebarSection) => void,
) => {
  const ActiveComponent =
    SECTIONS_CONFIG[section]?.component || SECTIONS_CONFIG.home.component;

  // Check if component accepts onNavigate (safely pass it to all setting components)
  // In a cleaner app we might have a specific type for ContentComponent
  return (
    <ActiveComponent
      onNavigate={(s: string) => onNavigate(s as SidebarSection)}
    />
  );
};

function App() {
  const { i18n, t } = useTranslation();
  const [showOnboarding, setShowOnboarding] = useState<boolean | null>(null);
  const [showAbout, setShowAbout] = useState(false);
  const [showUpgradeBanner, setShowUpgradeBanner] = useState(false);
  const upgradePromptShownRecordedRef = useRef(false);
  const direction = getLanguageDirection(i18n.language);
  const [currentSection, setCurrentSection] = useState<SidebarSection>("home");
  const previousSectionRef = useRef<SidebarSection>(currentSection);
  const settingsOpenedSourceRef = useRef<"sidebar" | "menu">("sidebar");
  const { settings, updateSetting } = useSettings();

  const navigateToSection = (
    section: SidebarSection,
    settingsSource: "sidebar" | "menu" = "sidebar",
  ) => {
    if (section === "settings") {
      settingsOpenedSourceRef.current = settingsSource;
    }
    setCurrentSection(section);
  };

  interface UndoMainToastPayload {
    kind: "feedback" | "discoverability_hint";
    code: string;
    shortcut?: string | null;
  }

  interface UpgradePromptEligibilityPayload {
    eligible: boolean;
    reason: string;
  }

  // Show window when the app is ready (prevents flash of white)
  useEffect(() => {
    commands.showMainWindow().catch((e: any) => {
      logError(`Failed to show main window: ${e}`, "App");
    });
  }, []);

  // Initialize unified logging system
  useEffect(() => {
    const cleanup = initLogging();
    return cleanup;
  }, []);

  // Initialize model store
  useEffect(() => {
    useModelStore.getState().initialize();
  }, []);

  // Initialize RTL direction when language changes
  useEffect(() => {
    initializeRTL(i18n.language);
  }, [i18n.language]);

  // Check onboarding status on mount and initialize shortcuts if complete
  useEffect(() => {
    checkOnboardingStatus();
  }, []);

  useEffect(() => {
    if (showOnboarding !== false) {
      return;
    }

    void consumePendingUpgradePromptOpenRequest("main_app_ready");
  }, [showOnboarding]);

  // Handle keyboard shortcuts for debug mode toggle
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Check for Ctrl+Shift+D (Windows/Linux) or Cmd+Shift+D (macOS)
      const isDebugShortcut =
        event.shiftKey &&
        event.key.toLowerCase() === "d" &&
        (event.ctrlKey || event.metaKey);

      if (isDebugShortcut) {
        event.preventDefault();
        const currentDebugMode = settings?.debug_mode ?? false;
        updateSetting("debug_mode", !currentDebugMode);
      }
    };

    // Add event listener when component mounts
    document.addEventListener("keydown", handleKeyDown);

    // Cleanup event listener when component unmounts
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [settings?.debug_mode, updateSetting]);

  // Listen for update check requests (e.g. from menu)
  useTauriEvent("check-for-updates", () => {
    const store = useUpdateStore.getState();

    if (store.isPendingRestart) {
      store.restartApp();
      return;
    }

    store.checkForUpdates(true);
    store.setShouldScrollToUpdates(true);
    navigateToSection("settings", "menu");
  });

  useEffect(() => {
    const previousSection = previousSectionRef.current;
    if (currentSection === "settings" && previousSection !== "settings") {
      void trackUiAnalyticsEvent("settings_opened", {
        source: settingsOpenedSourceRef.current,
      });
    }

    previousSectionRef.current = currentSection;
  }, [currentSection]);

  // Listen for about menu item
  useTauriEvent("open-about", () => {
    setShowAbout(true);
  });

  useTauriEvent<UpgradePromptEligibilityPayload>(
    "upgrade-prompt-eligible",
    (event) => {
      const payload = event.payload;
      if (!payload?.eligible) {
        return;
      }
      if (showOnboarding !== false) {
        return;
      }
      if (showUpgradeBanner) {
        return;
      }
      setShowUpgradeBanner(true);
    },
  );

  useTauriEvent("upgrade-prompt-open-requested", () => {
    if (showOnboarding !== false) {
      return;
    }
    void consumePendingUpgradePromptOpenRequest("upgrade_prompt_open_requested_event");
  });

  useTauriEvent<UndoMainToastPayload>("undo-main-toast", (event) => {
    const payload = event.payload;
    if (!payload) {
      return;
    }

    if (payload.kind === "feedback") {
      const messageMap: Record<string, string> = {
        undo_success: t("overlay.undo.feedback.success", "Undo applied"),
        undo_failed: t("overlay.undo.feedback.failed", "Undo failed"),
        undo_recording_canceled: t(
          "overlay.undo.feedback.recordingCanceled",
          "Recording canceled",
        ),
        undo_processing_canceled: t(
          "overlay.undo.feedback.processingCanceled",
          "Processing canceled",
        ),
        undo_noop_empty: t(
          "overlay.undo.feedback.nothingToUndo",
          "Nothing to undo",
        ),
        undo_noop_expired: t("overlay.undo.feedback.expired", "Undo expired"),
      };
      toast.message(
        messageMap[payload.code] ??
          t("overlay.undo.feedback.success", "Undo applied"),
      );
      return;
    }

    if (payload.kind === "discoverability_hint") {
      logInfo("event=undo_discoverability_hint_shown channel=main_toast", "App");
      commands
        .undoMarkDiscoverabilityHintSeen()
        .then(() => {
          logInfo(
            "event=undo_discoverability_hint_seen_marked channel=main_toast",
            "App",
          );
        })
        .catch((error) => {
          logError(
            `event=undo_discoverability_hint_seen_mark_failed channel=main_toast error=${error}`,
            "App",
          );
        });
      const shortcut =
        payload.shortcut ??
        t(
          "settings.general.shortcut.bindings.undo_last_transcript.name",
          "Undo last transcript",
        );
      toast.message(
        t(
          "overlay.undo.discoverability.hint",
          "Tip: Press {{shortcut}} to undo your last transcript within 2 minutes.",
          { shortcut },
        ),
      );
      return;
    }
  });

  // Listen for auto-switched microphone event
  useTauriEvent<{ previous: string; current: string }>(
    "audio-device-auto-switched",
    async (event) => {
      logInfo(
        `Audio device auto-switched: ${JSON.stringify(event.payload)}`,
        "App",
      );

      // Refresh settings and devices to reflect the change
      await useSettingsStore.getState().refreshSettings();
      await useSettingsStore.getState().refreshAudioDevices();

      toast.warning("Microphone Changed", {
        description: `Switched to ${event.payload.current} due to connection error with ${event.payload.previous}.`,
        duration: 5000,
      });
    },
  );

  const checkOnboardingStatus = async () => {
    try {
      // Check if onboarding was completed from user profile (separate from app settings)
      const profileResult = await commands.getUserProfileCommand();
      if (profileResult.status === "ok") {
        const userProfile = profileResult.data;
        if (userProfile.onboarding_completed) {
          void initializeInputAndShortcuts("onboarding_completed_check");

          // Refresh devices
          useSettingsStore.getState().refreshAudioDevices();
          useSettingsStore.getState().refreshOutputDevices();

          setShowOnboarding(false);
          return;
        }
      }
      // If not completed, show onboarding
      setShowOnboarding(true);
    } catch (error) {
      logError(`Failed to check onboarding status: ${error}`, "App");
      setShowOnboarding(true);
    }
  };

  const consumePendingUpgradePromptOpenRequest = async (source: string) => {
    try {
      const result = await commands.consumeUpgradePromptOpenRequest();
      if (result.status === "error") {
        logError(
          `event=upgrade_prompt_open_request_consume_failed source=${source} error=${result.error}`,
          "App",
        );
        return;
      }

      if (result.data) {
        logInfo(
          `event=upgrade_prompt_open_request_consumed source=${source}`,
          "App",
        );
        setShowUpgradeBanner(true);
      }
    } catch (error) {
      logError(
        `event=upgrade_prompt_open_request_consume_failed source=${source} error=${error}`,
        "App",
      );
    }
  };

  const initializeInputAndShortcuts = async (source: string) => {
    logInfo(`event=shortcut_init_attempt source=${source} channel=frontend`, "App");

    try {
      const [enigoResult, shortcutsResult] = await Promise.all([
        commands.initializeEnigo(),
        commands.initializeShortcuts(),
      ]);

      if (enigoResult.status === "error") {
        logError(
          `event=shortcut_init_failure source=${source} component=enigo error=${enigoResult.error}`,
          "App",
        );
      }

      if (shortcutsResult.status === "error") {
        logError(
          `event=shortcut_init_failure source=${source} component=shortcuts error=${shortcutsResult.error}`,
          "App",
        );
      }

      if (enigoResult.status === "ok" && shortcutsResult.status === "ok") {
        logInfo(
          `event=shortcut_init_success source=${source} channel=frontend`,
          "App",
        );
      }
    } catch (error) {
      logError(`event=shortcut_init_failure source=${source} error=${error}`, "App");
    }
  };

  const handleOnboardingComplete = () => {
    void initializeInputAndShortcuts("onboarding_completion_transition");
    // Transition to main app - onboarding is complete
    setShowOnboarding(false);
  };

  const recordUpgradePromptShown = async (source: string) => {
    try {
      const result = await commands.recordUpgradePromptShown("aha_moment", "v1");
      if (result.status === "error") {
        logError(
          `event=upgrade_prompt_shown_record_failed source=${source} error=${result.error}`,
          "App",
        );
      }
    } catch (error) {
      logError(
        `event=upgrade_prompt_shown_record_failed source=${source} error=${error}`,
        "App",
      );
    }
  };

  useEffect(() => {
    if (!showUpgradeBanner) {
      upgradePromptShownRecordedRef.current = false;
      return;
    }

    if (upgradePromptShownRecordedRef.current) {
      return;
    }

    upgradePromptShownRecordedRef.current = true;
    void recordUpgradePromptShown("upgrade_banner_visible");
  }, [showUpgradeBanner]);

  const handleUpgradeBannerDismiss = async () => {
    try {
      const result = await commands.recordUpgradePromptAction(
        "dismissed",
        "aha_moment",
      );
      if (result.status === "error") {
        logError(
          `event=upgrade_prompt_action_record_failed action=dismissed error=${result.error}`,
          "App",
        );
      }
    } catch (error) {
      logError(
        `event=upgrade_prompt_action_record_failed action=dismissed error=${error}`,
        "App",
      );
    }
    setShowUpgradeBanner(false);
  };

  const handleUpgradeBannerClick = async () => {
    setShowUpgradeBanner(false);

    // Open pricing immediately; analytics calls run in the background.
    try {
      await openUrl("https://codictate.com/pricing");
    } catch (error) {
      logError(`event=upgrade_url_open_failed error=${error}`, "App");
    }

    void commands
      .recordUpgradePromptAction("cta_clicked", "aha_moment")
      .then((actionResult) => {
        if (actionResult.status === "error") {
          logError(
            `event=upgrade_prompt_action_record_failed action=cta_clicked error=${actionResult.error}`,
            "App",
          );
        }
      })
      .catch((error) => {
        logError(
          `event=upgrade_prompt_action_record_failed action=cta_clicked error=${error}`,
          "App",
        );
      });

    void commands
      .recordUpgradeCheckoutResult("started", "aha_prompt")
      .then((checkoutResult) => {
        if (checkoutResult.status === "error") {
          logError(
            `event=upgrade_checkout_result_record_failed result=started error=${checkoutResult.error}`,
            "App",
          );
        }
      })
      .catch((error) => {
        logError(
          `event=upgrade_checkout_result_record_failed result=started error=${error}`,
          "App",
        );
      });
  };

  if (showOnboarding) {
    return <Onboarding onComplete={handleOnboardingComplete} />;
  }

  return (
    <SidebarProvider
      style={
        {
          "--sidebar-width": "10rem",
          "--sidebar-width-icon": "4rem",
        } as React.CSSProperties
      }
      className="h-screen w-full overflow-hidden"
      dir={direction}
    >
      <Toaster
        theme="system"
        toastOptions={{
          unstyled: true,
          classNames: {
            toast:
              "bg-background border border-mid-gray/20 rounded-lg shadow-lg px-4 py-3 flex items-center gap-3 text-sm",
            title: "font-medium",
            description: "text-mid-gray",
          },
        }}
      />
      <Sidebar
        activeSection={currentSection}
        onSectionChange={(section) => navigateToSection(section, "sidebar")}
      />
      <SidebarInset>
        {/* Main content area */}
        <div className="flex-1 flex flex-col overflow-hidden relative">
          <div className="absolute left-4 top-4 z-50">
            <SidebarTrigger />
          </div>
          <div
            className={`flex-1 flex flex-col ${
              currentSection === "history" || currentSection === "dictionary"
                ? "overflow-hidden"
                : "overflow-y-auto"
            }`}
          >
            <div className="flex-1 flex flex-col items-center gap-4 min-h-0 w-full">
              <UpgradePromptBanner
                visible={showUpgradeBanner}
                onUpgrade={handleUpgradeBannerClick}
                onDismiss={handleUpgradeBannerDismiss}
              />
              <AccessibilityPermissions />
              <MicrophonePermissions />
              {renderSettingsContent(currentSection, (section) =>
                navigateToSection(section, "sidebar"),
              )}
            </div>
          </div>
        </div>
      </SidebarInset>
      <AboutModal open={showAbout} onOpenChange={setShowAbout} />
    </SidebarProvider>
  );
}

export default App;
