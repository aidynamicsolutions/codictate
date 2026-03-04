import { describe, expect, it, vi } from "vitest";

import {
  BACKUP_SUCCESS_TOAST_DURATION_MS,
  BACKUP_UNENCRYPTED_TOAST_DURATION_MS,
  showBackupSuccessToastSequence,
} from "./backupToastSequencing";

type ToastOptions = {
  id?: string | number;
  duration?: number;
  description?: string;
  onAutoClose?: (toast: unknown) => void;
  onDismiss?: (toast: unknown) => void;
};

function createToastMock() {
  const success = vi.fn((_message: string, _data?: ToastOptions) => 1);
  const message = vi.fn((_message: string, _data?: ToastOptions) => 2);
  return { success, message };
}

describe("showBackupSuccessToastSequence", () => {
  it("shows success immediately and does not show warning immediately", () => {
    const toastApi = createToastMock();

    showBackupSuccessToastSequence({
      successTitle: "Backup created",
      successDescription: "Saved to /tmp/test.codictatebackup",
      unencryptedInfo: "Backups are not encrypted in v1.",
      toastApi,
    });

    expect(toastApi.success).toHaveBeenCalledTimes(1);
    expect(toastApi.message).not.toHaveBeenCalled();
  });

  it("shows warning once when success toast auto-closes", () => {
    const toastApi = createToastMock();

    showBackupSuccessToastSequence({
      successTitle: "Backup created",
      successDescription: "Saved to /tmp/test.codictatebackup",
      unencryptedInfo: "Backups are not encrypted in v1.",
      toastApi,
    });

    const successOptions = toastApi.success.mock.calls[0][1] as ToastOptions;
    successOptions.onAutoClose?.({});

    expect(toastApi.message).toHaveBeenCalledTimes(1);
    expect(toastApi.message).toHaveBeenCalledWith("Backups are not encrypted in v1.", {
      id: "backup-export-unencrypted-info",
      duration: BACKUP_UNENCRYPTED_TOAST_DURATION_MS,
    });
  });

  it("shows warning once when success toast is dismissed manually", () => {
    const toastApi = createToastMock();

    showBackupSuccessToastSequence({
      successTitle: "Backup created",
      successDescription: "Saved to /tmp/test.codictatebackup",
      unencryptedInfo: "Backups are not encrypted in v1.",
      toastApi,
    });

    const successOptions = toastApi.success.mock.calls[0][1] as ToastOptions;
    successOptions.onDismiss?.({});

    expect(toastApi.message).toHaveBeenCalledTimes(1);
  });

  it("shows warning only once if auto-close and dismiss both fire", () => {
    const toastApi = createToastMock();

    showBackupSuccessToastSequence({
      successTitle: "Backup created",
      successDescription: "Saved to /tmp/test.codictatebackup",
      unencryptedInfo: "Backups are not encrypted in v1.",
      toastApi,
    });

    const successOptions = toastApi.success.mock.calls[0][1] as ToastOptions;
    successOptions.onAutoClose?.({});
    successOptions.onDismiss?.({});

    expect(toastApi.message).toHaveBeenCalledTimes(1);
  });

  it("uses configured durations and IDs", () => {
    const toastApi = createToastMock();

    showBackupSuccessToastSequence({
      successTitle: "Backup created",
      successDescription: "Saved to /tmp/test.codictatebackup",
      unencryptedInfo: "Backups are not encrypted in v1.",
      toastApi,
    });

    const successOptions = toastApi.success.mock.calls[0][1] as ToastOptions;
    expect(successOptions.description).toBe("Saved to /tmp/test.codictatebackup");
    expect(successOptions.duration).toBe(BACKUP_SUCCESS_TOAST_DURATION_MS);
    expect(successOptions.id).toBe("backup-export-success");
    expect(successOptions.onAutoClose).toBeTypeOf("function");
    expect(successOptions.onDismiss).toBeTypeOf("function");
  });
});
