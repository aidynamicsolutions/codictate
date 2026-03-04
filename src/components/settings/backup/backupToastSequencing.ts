import { toast, type ExternalToast } from "sonner";

export const BACKUP_SUCCESS_TOAST_DURATION_MS = 6500;
export const BACKUP_UNENCRYPTED_TOAST_DURATION_MS = 7000;
const BACKUP_SUCCESS_TOAST_ID = "backup-export-success";
const BACKUP_UNENCRYPTED_TOAST_ID = "backup-export-unencrypted-info";

type BackupToastApi = {
  success: (message: string, data?: ExternalToast) => string | number;
  message: (message: string, data?: ExternalToast) => string | number;
};

interface BackupSuccessToastSequenceInput {
  successTitle: string;
  successDescription: string;
  unencryptedInfo: string;
  toastApi?: BackupToastApi;
}

export function showBackupSuccessToastSequence({
  successTitle,
  successDescription,
  unencryptedInfo,
  toastApi = toast,
}: BackupSuccessToastSequenceInput): void {
  let warningShown = false;

  const showUnencryptedWarning = () => {
    if (warningShown) {
      return;
    }
    warningShown = true;
    toastApi.message(unencryptedInfo, {
      id: BACKUP_UNENCRYPTED_TOAST_ID,
      duration: BACKUP_UNENCRYPTED_TOAST_DURATION_MS,
    });
  };

  toastApi.success(successTitle, {
    description: successDescription,
    duration: BACKUP_SUCCESS_TOAST_DURATION_MS,
    id: BACKUP_SUCCESS_TOAST_ID,
    onAutoClose: showUnencryptedWarning,
    onDismiss: showUnencryptedWarning,
  });
}
