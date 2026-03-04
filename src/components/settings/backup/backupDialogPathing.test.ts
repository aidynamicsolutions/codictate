import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  BACKUP_LAST_DIRECTORY_STORAGE_KEY,
  buildSuggestedBackupFileName,
  buildBackupSaveDefaultPath,
  extractParentDirectoryFromPath,
  formatBackupTimestamp,
  getBackupOpenDefaultPath,
  getRememberedBackupDirectory,
  rememberBackupDirectoryFromFilePath,
} from "./backupDialogPathing";

type StorageMock = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
  clear: () => void;
};

function createStorageMock(): StorageMock {
  const data = new Map<string, string>();
  return {
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => {
      data.set(key, value);
    },
    removeItem: (key) => {
      data.delete(key);
    },
    clear: () => {
      data.clear();
    },
  };
}

describe("backupDialogPathing", () => {
  let storageMock: StorageMock;

  beforeEach(() => {
    storageMock = createStorageMock();
    vi.stubGlobal("localStorage", storageMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("extracts parent directory from unix-style file paths", () => {
    const directory = extractParentDirectoryFromPath(
      "/Users/tiger/Backups/passA-complete.codictatebackup",
    );

    expect(directory).toBe("/Users/tiger/Backups");
  });

  it("extracts parent directory from windows-style file paths", () => {
    const directory = extractParentDirectoryFromPath(
      "C:\\Users\\Tiger\\Backups\\passA-smaller.codictatebackup",
    );

    expect(directory).toBe("C:/Users/Tiger/Backups");
  });

  it("returns safe fallback values for missing or invalid inputs", () => {
    expect(extractParentDirectoryFromPath("")).toBeNull();
    expect(extractParentDirectoryFromPath(null)).toBeNull();

    storageMock.setItem(BACKUP_LAST_DIRECTORY_STORAGE_KEY, "invalid-path");
    expect(getRememberedBackupDirectory()).toBeNull();
    expect(getBackupOpenDefaultPath()).toBeUndefined();
  });

  it("builds save default path using remembered folder", () => {
    storageMock.setItem(BACKUP_LAST_DIRECTORY_STORAGE_KEY, "/Users/tiger/Backups");

    expect(
      buildBackupSaveDefaultPath("codictate-complete-backup.codictatebackup"),
    ).toBe("/Users/tiger/Backups/codictate-complete-backup.codictatebackup");
  });

  it("persists the remembered folder from selected file paths", () => {
    rememberBackupDirectoryFromFilePath(
      "/Users/tiger/Desktop/Backups/passB-complete.codictatebackup",
    );

    expect(getRememberedBackupDirectory()).toBe("/Users/tiger/Desktop/Backups");
    expect(getBackupOpenDefaultPath()).toBe("/Users/tiger/Desktop/Backups");
  });

  it("formats backup timestamp as DD-MM-YYYY-HHmm", () => {
    const date = new Date(2026, 2, 3, 14, 25, 0, 0);
    expect(formatBackupTimestamp(date)).toBe("03-03-2026-1425");
  });

  it("builds scope-specific suggested backup file names", () => {
    const date = new Date(2026, 2, 3, 14, 25, 0, 0);
    expect(buildSuggestedBackupFileName("complete", date)).toBe(
      "codictate-complete-backup-03-03-2026-1425.codictatebackup",
    );
    expect(buildSuggestedBackupFileName("smaller", date)).toBe(
      "codictate-smaller-backup-03-03-2026-1425.codictatebackup",
    );
  });
});
