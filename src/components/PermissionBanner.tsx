import { useEffect, useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertCircle } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import PermissionModal, { PermissionType } from "./PermissionModal";

interface PermissionBannerProps {
  type: PermissionType;
  checkPermission: () => Promise<boolean>;
  eventName: string;
  settingsUrl: string;
  /** Called when permission transitions from denied to granted */
  onPermissionGranted?: () => void;
}

/**
 * Generic permission banner component that handles:
 * - Permission checking on mount and focus
 * - Listening for backend permission events
 * - Showing modal when permission denied
 * - Showing persistent banner after modal dismissed
 * - Hiding when permission granted
 */
const PermissionBanner: React.FC<PermissionBannerProps> = ({
  type,
  checkPermission,
  eventName,
  settingsUrl,
  onPermissionGranted,
}) => {
  const { t } = useTranslation();
  const [hasPermission, setHasPermission] = useState<boolean | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [modalDismissed, setModalDismissed] = useState(false);

  // Permission check function - reused on mount and focus
  const checkPermissions = useCallback(async () => {
    const granted = await checkPermission();
    const wasGranted = hasPermission;
    setHasPermission(granted);

    if (granted) {
      // Permission granted - hide modal/banner and reset dismissed state
      setShowModal(false);
      setModalDismissed(false);

      // Call onPermissionGranted if transitioning from denied to granted
      if (wasGranted === false && onPermissionGranted) {
        onPermissionGranted();
      }
    }
    // Note: Modal is shown via event listener, not here (except initial mount)
  }, [checkPermission, hasPermission, onPermissionGranted]);

  // Check permissions on mount
  useEffect(() => {
    const initialCheck = async () => {
      const granted = await checkPermission();
      setHasPermission(granted);
      // Show modal on initial mount if permission denied
      if (!granted) {
        setShowModal(true);
      }
    };
    initialCheck();
  }, [checkPermission]);

  // Re-check permissions when window regains focus (user returns from System Settings)
  useEffect(() => {
    const handleFocus = () => {
      checkPermissions();
    };

    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, [checkPermissions]);

  // Listen for permission event from backend
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen(eventName, () => {
        setHasPermission(false);
        if (!modalDismissed) {
          setShowModal(true);
        }
      });
    };

    setupListener();

    return () => {
      unlisten?.();
    };
  }, [eventName, modalDismissed]);

  // Handle modal dismiss - show persistent banner instead
  const handleModalDismiss = (open: boolean) => {
    if (!open) {
      setModalDismissed(true);
      setShowModal(false);
    }
  };

  const handleOpenSettings = async () => {
    await openUrl(settingsUrl);
  };

  // Still loading or permission granted - don't show anything
  if (hasPermission === null || hasPermission) {
    return null;
  }

  // Show modal if not dismissed yet
  if (showModal) {
    return (
      <PermissionModal
        open={showModal}
        onOpenChange={handleModalDismiss}
        type={type}
      />
    );
  }

  // Show persistent error banner after modal dismissed
  if (!modalDismissed) {
    return null;
  }

  return (
    <div className="p-4 w-full rounded-lg border border-destructive bg-destructive/10">
      <div className="flex justify-between items-center gap-3">
        <div className="flex items-center gap-3">
          <AlertCircle className="h-5 w-5 text-destructive flex-shrink-0" />
          <p className="text-sm font-medium text-destructive-foreground">
            {t(`${type}.permissionsDescription`, {
              appName: t("appName"),
            })}
          </p>
        </div>
        <button
          onClick={handleOpenSettings}
          className="min-h-10 px-3 py-1.5 text-sm font-medium bg-destructive text-destructive-foreground hover:bg-destructive/90 rounded cursor-pointer whitespace-nowrap"
        >
          {t(`${type}.openSettings`)}
        </button>
      </div>
    </div>
  );
};

export default PermissionBanner;
