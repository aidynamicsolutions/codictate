import { useEffect, useState } from "react";
import { Toaster, toast } from "sonner";
import { useTranslation } from "react-i18next";
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
import { useModelStore } from "./stores/modelStore";
import { useSettingsStore } from "./stores/settingsStore";
import { useUpdateStore } from "./stores/updateStore";
import { AboutModal } from "./components/AboutModal";

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
  const direction = getLanguageDirection(i18n.language);
  const [currentSection, setCurrentSection] = useState<SidebarSection>("home");
  const { settings, updateSetting } = useSettings();

  interface UndoMainToastPayload {
    kind: "feedback" | "discoverability_hint";
    code: string;
    shortcut?: string | null;
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
    setCurrentSection("settings");
  });

  // Listen for about menu item
  useTauriEvent("open-about", () => {
    setShowAbout(true);
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
          // Initialize shortcuts when onboarding is already complete
          // (During onboarding, this is called from PermissionsStep.tsx)
          Promise.all([
            commands.initializeEnigo(),
            commands.initializeShortcuts(),
          ]).catch((e) => {
            logError(`Failed to initialize: ${e}`, "App");
          });

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

  const handleOnboardingComplete = () => {
    // Transition to main app - onboarding is complete
    setShowOnboarding(false);
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
        onSectionChange={setCurrentSection}
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
              <AccessibilityPermissions />
              <MicrophonePermissions />
              {renderSettingsContent(currentSection, setCurrentSection)}
            </div>
          </div>
        </div>
      </SidebarInset>
      <AboutModal open={showAbout} onOpenChange={setShowAbout} />
    </SidebarProvider>
  );
}

export default App;
