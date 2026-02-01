import { useEffect, useState } from "react";
import { Toaster } from "sonner";
import "./App.css";
import AccessibilityPermissions from "./components/AccessibilityPermissions";
import MicrophonePermissions from "./components/MicrophonePermissions";
import Footer from "./components/footer";
import Onboarding from "./components/onboarding";
import { Sidebar, SidebarSection, SECTIONS_CONFIG } from "./components/Sidebar";
import { useSettings } from "./hooks/useSettings";
import { commands } from "@/bindings";
import { initLogging } from "@/utils/logging";
import { useModelStore } from "./stores/modelStore";

const renderSettingsContent = (
  section: SidebarSection,
  onNavigate: (section: SidebarSection) => void
) => {
  const ActiveComponent =
    SECTIONS_CONFIG[section]?.component || SECTIONS_CONFIG.home.component;

  // Check if component accepts onNavigate (safely pass it to all setting components)
  // In a cleaner app we might have a specific type for ContentComponent
  return <ActiveComponent onNavigate={(s: string) => onNavigate(s as SidebarSection)} />;
};

function App() {
  const [showOnboarding, setShowOnboarding] = useState<boolean | null>(null);
  const [currentSection, setCurrentSection] =
    useState<SidebarSection>("home");
  const { settings, updateSetting } = useSettings();

  // Show window when the app is ready (prevents flash of white)
  useEffect(() => {
    commands.showMainWindow().catch((e: any) => {
      console.error("Failed to show main window:", e);
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

  useEffect(() => {
    checkOnboardingStatus();
  }, []);

  // Note: Permission event listeners are now handled by AccessibilityPermissions and MicrophonePermissions components

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

  const checkOnboardingStatus = async () => {
    try {
      // Check if onboarding was completed from user profile (separate from app settings)
      const profileResult = await commands.getUserProfileCommand();
      if (profileResult.status === "ok") {
        const userProfile = profileResult.data;
        if (userProfile.onboarding_completed) {
          setShowOnboarding(false);
          return;
        }
      }
      // If not completed, show onboarding
      setShowOnboarding(true);
    } catch (error) {
      console.error("Failed to check onboarding status:", error);
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
    <div className="h-screen flex flex-col select-none cursor-default">
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
      {/* Main content area that takes remaining space */}
      <div className="flex-1 flex overflow-hidden">
        <Sidebar
          activeSection={currentSection}
          onSectionChange={setCurrentSection}
        />
        {/* Scrollable content area */}
        <div className="flex-1 flex flex-col overflow-hidden">
          <div
            className={`flex-1 flex flex-col ${
              currentSection === "history" ? "overflow-hidden" : "overflow-y-auto"
            }`}
          >
            <div className="flex-1 flex flex-col items-center p-4 gap-4 min-h-0 w-full">
              <AccessibilityPermissions />
              <MicrophonePermissions />
              {renderSettingsContent(currentSection, setCurrentSection)}
            </div>
          </div>
        </div>
      </div>
      {/* Fixed footer at bottom */}
      <Footer />
    </div>
  );
}

export default App;
