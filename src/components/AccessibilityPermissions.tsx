import { checkAccessibilityPermission } from "tauri-plugin-macos-permissions-api";
import PermissionBanner from "./PermissionBanner";
import { commands } from "@/bindings";
import { logError, logInfo } from "@/utils/logging";

const SETTINGS_URL =
  "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

const AccessibilityPermissions: React.FC = () => {
  const handlePermissionGranted = () => {
    void (async () => {
      const source = "accessibility_permission_granted";
      logInfo(`event=shortcut_init_attempt source=${source} channel=frontend`, "AccessibilityPermissions");

      try {
        const enigoResult = await commands.initializeEnigo();
        if (enigoResult.status === "error") {
          logError(
            `event=shortcut_init_failure source=${source} component=enigo error=${enigoResult.error}`,
            "AccessibilityPermissions",
          );
        }

        const shortcutsResult = await commands.initializeShortcuts();
        if (shortcutsResult.status === "error") {
          logError(
            `event=shortcut_init_failure source=${source} component=shortcuts error=${shortcutsResult.error}`,
            "AccessibilityPermissions",
          );
        }

        const fnMonitorResult = await commands.startFnKeyMonitor(true);
        if (fnMonitorResult.status === "error") {
          logError(
            `event=shortcut_init_failure source=${source} component=fn_monitor error=${fnMonitorResult.error}`,
            "AccessibilityPermissions",
          );
        }

        if (
          enigoResult.status === "ok" &&
          shortcutsResult.status === "ok" &&
          fnMonitorResult.status === "ok"
        ) {
          logInfo(
            `event=shortcut_init_success source=${source} channel=frontend`,
            "AccessibilityPermissions",
          );
        }
      } catch (error) {
        logError(
          `event=shortcut_init_failure source=${source} error=${error}`,
          "AccessibilityPermissions",
        );
      }
    })();
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
