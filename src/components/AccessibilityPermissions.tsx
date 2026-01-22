import { checkAccessibilityPermission } from "tauri-plugin-macos-permissions-api";
import PermissionBanner from "./PermissionBanner";
import { commands } from "@/bindings";

const SETTINGS_URL =
  "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

const AccessibilityPermissions: React.FC = () => {
  const handlePermissionGranted = () => {
    // Restart Fn key monitor when accessibility permission is granted
    commands.startFnKeyMonitor(true).catch((err) => {
      console.error("Failed to restart Fn key monitor:", err);
    });
  };

  return (
    <PermissionBanner
      type="accessibility"
      checkPermission={checkAccessibilityPermission}
      eventName="accessibility-permission-lost"
      settingsUrl={SETTINGS_URL}
      onPermissionGranted={handlePermissionGranted}
    />
  );
};

export default AccessibilityPermissions;
