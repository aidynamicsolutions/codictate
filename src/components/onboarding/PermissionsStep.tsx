import React, { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, Check, Info, ArrowLeft } from "lucide-react";
import { Button } from "@/components/shared/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import OnboardingLayout from "./OnboardingLayout";
import SkipConfirmationModal from "./SkipConfirmationModal";
import {
  checkAccessibilityPermission,
  requestAccessibilityPermission,
  checkMicrophonePermission,
  requestMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { openUrl } from "@tauri-apps/plugin-opener";

// Permission states
type PermissionStatus = "idle" | "checking" | "granted";

// Polling configuration
const POLL_INTERVAL_MS = 500;
const POLL_TIMEOUT_MS = 60000;

// System Settings URLs for macOS
const SYSTEM_SETTINGS_URLS = {
  microphone: "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
};

interface PermissionsStepProps {
  onContinue: () => void;
  onBack?: () => void;
}

interface PermissionCardProps {
  title: string;
  description: string;
  tooltipText: string;
  successText: string;
  status: PermissionStatus;
  onAllow: () => void;
  allowText: string;
  retryText: string;
  showRetry: boolean;
}

const PermissionCard: React.FC<PermissionCardProps> = ({
  title,
  description,
  tooltipText,
  successText,
  status,
  onAllow,
  allowText,
  retryText,
  showRetry,
}) => {
  const isGranted = status === "granted";
  const isChecking = status === "checking";

  return (
    <div
      className={`rounded-lg border p-4 transition-all ${
        isGranted
          ? "border-green-500/50 bg-green-50 dark:bg-green-950/20"
          : "border-border bg-background"
      }`}
    >
      {isGranted ? (
        // Success state
        <div className="flex items-center gap-3">
          <Check className="h-5 w-5 text-green-600 dark:text-green-400" />
          <span className="text-sm font-medium text-green-700 dark:text-green-300">
            {successText}
          </span>
        </div>
      ) : (
        // Idle/Checking state
        <div className="flex flex-col gap-3">
          <div className="flex items-start justify-between">
            <div className="flex flex-col gap-1">
              <span className="font-medium text-foreground">{title}</span>
              <span className="text-sm text-muted-foreground">
                {description}
              </span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button
              onClick={onAllow}
              disabled={isChecking}
              size="sm"
              className="w-fit"
            >
              {showRetry ? retryText : allowText}
              {isChecking && <Loader2 className="ml-2 h-4 w-4 animate-spin" />}
            </Button>

            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  className="rounded-full p-1 text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
                >
                  <Info className="h-4 w-4" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right" className="max-w-xs">
                <p>{tooltipText}</p>
              </TooltipContent>
            </Tooltip>
          </div>
        </div>
      )}
    </div>
  );
};

export const PermissionsStep: React.FC<PermissionsStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();

  // Permission states
  const [accessibilityStatus, setAccessibilityStatus] =
    useState<PermissionStatus>("idle");
  const [microphoneStatus, setMicrophoneStatus] =
    useState<PermissionStatus>("idle");

  // Skip confirmation modal state
  const [showSkipModal, setShowSkipModal] = useState(false);

  // Retry flags (shown after timeout)
  const [accessibilityShowRetry, setAccessibilityShowRetry] = useState(false);
  const [microphoneShowRetry, setMicrophoneShowRetry] = useState(false);

  // Track if microphone was previously denied (need to open settings manually)
  const [microphonePreviouslyDenied, setMicrophonePreviouslyDenied] = useState(false);

  // Polling refs
  const accessibilityPollRef = useRef<NodeJS.Timeout | null>(null);
  const microphonePollRef = useRef<NodeJS.Timeout | null>(null);
  const accessibilityTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const microphoneTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Check initial permission states on mount
  useEffect(() => {
    const checkInitialPermissions = async () => {
      try {
        const hasAccessibility = await checkAccessibilityPermission();
        if (hasAccessibility) {
          setAccessibilityStatus("granted");
        }
        const hasMicrophone = await checkMicrophonePermission();
        if (hasMicrophone) {
          setMicrophoneStatus("granted");
        }
      } catch (error) {
        console.error("Error checking initial permissions:", error);
      }
    };
    checkInitialPermissions();

    // Cleanup on unmount
    return () => {
      if (accessibilityPollRef.current) clearInterval(accessibilityPollRef.current);
      if (microphonePollRef.current) clearInterval(microphonePollRef.current);
      if (accessibilityTimeoutRef.current) clearTimeout(accessibilityTimeoutRef.current);
      if (microphoneTimeoutRef.current) clearTimeout(microphoneTimeoutRef.current);
    };
  }, []);

  // Polling function for accessibility
  const startAccessibilityPolling = useCallback(() => {
    // Clear any existing polling
    if (accessibilityPollRef.current) clearInterval(accessibilityPollRef.current);
    if (accessibilityTimeoutRef.current) clearTimeout(accessibilityTimeoutRef.current);

    setAccessibilityStatus("checking");
    setAccessibilityShowRetry(false);

    // Start polling
    accessibilityPollRef.current = setInterval(async () => {
      try {
        const hasPermission = await checkAccessibilityPermission();
        if (hasPermission) {
          if (accessibilityPollRef.current) clearInterval(accessibilityPollRef.current);
          if (accessibilityTimeoutRef.current) clearTimeout(accessibilityTimeoutRef.current);
          setAccessibilityStatus("granted");
        }
      } catch (error) {
        console.error("Error polling accessibility permission:", error);
      }
    }, POLL_INTERVAL_MS);

    // Set timeout
    accessibilityTimeoutRef.current = setTimeout(() => {
      if (accessibilityPollRef.current) clearInterval(accessibilityPollRef.current);
      setAccessibilityStatus("idle");
      setAccessibilityShowRetry(true);
    }, POLL_TIMEOUT_MS);
  }, []);

  // Polling function for microphone
  const startMicrophonePolling = useCallback(() => {
    // Clear any existing polling
    if (microphonePollRef.current) clearInterval(microphonePollRef.current);
    if (microphoneTimeoutRef.current) clearTimeout(microphoneTimeoutRef.current);

    setMicrophoneStatus("checking");
    setMicrophoneShowRetry(false);

    // Start polling
    microphonePollRef.current = setInterval(async () => {
      try {
        const hasPermission = await checkMicrophonePermission();
        if (hasPermission) {
          if (microphonePollRef.current) clearInterval(microphonePollRef.current);
          if (microphoneTimeoutRef.current) clearTimeout(microphoneTimeoutRef.current);
          setMicrophoneStatus("granted");
        }
      } catch (error) {
        console.error("Error polling microphone permission:", error);
      }
    }, POLL_INTERVAL_MS);

    // Set timeout
    microphoneTimeoutRef.current = setTimeout(() => {
      if (microphonePollRef.current) clearInterval(microphonePollRef.current);
      setMicrophoneStatus("idle");
      setMicrophoneShowRetry(true);
      // Mark as previously denied so next attempt opens settings directly
      setMicrophonePreviouslyDenied(true);
    }, POLL_TIMEOUT_MS);
  }, []);

  // Handle accessibility permission request
  const handleAccessibilityAllow = async () => {
    try {
      await requestAccessibilityPermission();
      startAccessibilityPolling();
    } catch (error) {
      console.error("Error requesting accessibility permission:", error);
      startAccessibilityPolling();
    }
  };

  // Handle microphone permission request
  const handleMicrophoneAllow = async () => {
    try {
      // If previously denied or on retry, open System Settings directly
      // because macOS won't show the native dialog again
      if (microphonePreviouslyDenied || microphoneShowRetry) {
        await openUrl(SYSTEM_SETTINGS_URLS.microphone);
        startMicrophonePolling();
        return;
      }

      // First attempt: try the native permission request
      // This will show a system dialog if permission was never requested
      await requestMicrophonePermission();
      
      // Check immediately if permission was granted via the dialog
      const hasPermission = await checkMicrophonePermission();
      if (hasPermission) {
        setMicrophoneStatus("granted");
        return;
      }
      
      // If still no permission, the dialog might have been dismissed or denied
      // Open System Settings and start polling
      setMicrophonePreviouslyDenied(true);
      await openUrl(SYSTEM_SETTINGS_URLS.microphone);
      startMicrophonePolling();
    } catch (error) {
      console.error("Error requesting microphone permission:", error);
      // Fallback: try to open System Settings directly
      try {
        await openUrl(SYSTEM_SETTINGS_URLS.microphone);
      } catch (openError) {
        console.error("Error opening System Settings:", openError);
      }
      startMicrophonePolling();
    }
  };

  // Both permissions required to continue
  const canContinue =
    accessibilityStatus === "granted" && microphoneStatus === "granted";

  // Determine which video to show based on current permission progress
  const getVideoSource = () => {
    if (accessibilityStatus !== "granted") {
      // Show accessibility video
      return {
        webm: "/src-tauri/resources/videos/accessibilityPermission.webm",
        mp4: "/src-tauri/resources/videos/accessibilityPermission.mp4",
      };
    }
    // Show microphone video
    return {
      webm: "/src-tauri/resources/videos/micPermission11.57.35am_compressed.webm",
      mp4: null,
    };
  };

  const videoSource = getVideoSource();

  return (
    <>
    <OnboardingLayout
      currentStep="permissions"
      leftContent={
        <div className="flex flex-col h-full">
          {/* Back button - positioned at top */}
          {onBack && (
            <button
              type="button"
              onClick={onBack}
              className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors w-fit mb-auto"
            >
              <ArrowLeft className="h-4 w-4" />
              {t("onboarding.permissions.back")}
            </button>
          )}

          {/* Content centered vertically */}
          <div className="flex flex-col gap-6 my-auto">
          <div className="flex flex-col gap-2 mb-8">
            <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl max-w-[380px]">
              {t("onboarding.permissions.title", { appName: t("appName") })}
            </h1>
          </div>

          {/* Accessibility Permission Card */}
          <PermissionCard
            title={t("onboarding.permissions.accessibility.title", {
              appName: t("appName"),
            })}
            description={t("onboarding.permissions.accessibility.description", {
              appName: t("appName"),
            })}
            tooltipText={t("onboarding.permissions.accessibility.tooltip", {
              appName: t("appName"),
            })}
            successText={t("onboarding.permissions.accessibility.success", {
              appName: t("appName"),
            })}
            status={accessibilityStatus}
            onAllow={handleAccessibilityAllow}
            allowText={t("onboarding.permissions.allow")}
            retryText={t("onboarding.permissions.retry")}
            showRetry={accessibilityShowRetry}
          />

          {/* Microphone Permission Card */}
          <PermissionCard
            title={t("onboarding.permissions.microphone.title", {
              appName: t("appName"),
            })}
            description={t("onboarding.permissions.microphone.description", {
              appName: t("appName"),
            })}
            tooltipText={t("onboarding.permissions.microphone.tooltip", {
              appName: t("appName"),
            })}
            successText={t("onboarding.permissions.microphone.success", {
              appName: t("appName"),
            })}
            status={microphoneStatus}
            onAllow={handleMicrophoneAllow}
            allowText={t("onboarding.permissions.allow")}
            retryText={t("onboarding.permissions.retry")}
            showRetry={microphoneShowRetry}
          />
          </div>

          {/* Continue button at bottom */}
          <Button
            onClick={() => {
              if (canContinue) {
                onContinue();
              } else {
                setShowSkipModal(true);
              }
            }}
            size="lg"
            className="mt-auto w-fit"
          >
            {t("onboarding.permissions.continue")}
          </Button>
        </div>
      }
      rightContent={
        <div className="flex items-center justify-center h-full">
          <video
            key={videoSource.webm}
            autoPlay
            loop
            muted
            playsInline
            className="h-auto max-h-[500px] w-auto max-w-full rounded-lg shadow-lg"
          >
            <source src={videoSource.webm} type="video/webm" />
            {videoSource.mp4 && (
              <source src={videoSource.mp4} type="video/mp4" />
            )}
          </video>
        </div>
      }
    />

    {/* Skip confirmation modal */}
    <SkipConfirmationModal
      open={showSkipModal}
      onOpenChange={setShowSkipModal}
      onConfirm={() => {
        setShowSkipModal(false);
        onContinue();
      }}
      onCancel={() => setShowSkipModal(false)}
    />
    </>
  );
};

export default PermissionsStep;
