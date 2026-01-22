import { useEffect, useState } from "react";
import {
  checkMicrophonePermission,
  checkAccessibilityPermission,
} from "tauri-plugin-macos-permissions-api";
import PermissionBanner from "./PermissionBanner";

const SETTINGS_URL =
  "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone";

const MicrophonePermissions: React.FC = () => {
  const [accessibilityGranted, setAccessibilityGranted] = useState<
    boolean | null
  >(null);

  // Check accessibility permission on mount and focus
  // Only show microphone banner when accessibility is already granted
  useEffect(() => {
    const check = async () => {
      const granted = await checkAccessibilityPermission();
      setAccessibilityGranted(granted);
    };
    check();

    const handleFocus = () => check();
    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, []);

  // Wait for accessibility check, or if accessibility not granted, don't show
  if (accessibilityGranted !== true) {
    return null;
  }

  return (
    <PermissionBanner
      type="microphone"
      checkPermission={checkMicrophonePermission}
      eventName="microphone-permission-denied"
      settingsUrl={SETTINGS_URL}
    />
  );
};

export default MicrophonePermissions;
