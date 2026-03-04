import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  BACKUP_ETA_HISTORY_STORAGE_KEY,
  etaBucketKeyForRemainingSeconds,
  isBackupRestoreCancelledError,
  loadBackupEtaHistory,
  persistBackupEtaHistorySample,
  predictTotalDurationSeconds,
} from "./backupProgressModel";

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

describe("predictTotalDurationSeconds", () => {
  it("returns conservative fallback for large complete backups", () => {
    const predicted = predictTotalDurationSeconds(
      {
        scope: "complete",
        estimated_size_bytes: 6 * 1024 * 1024 * 1024,
        recording_files: 9000,
        history_entries: 400_000,
      },
      [],
    );
    expect(predicted).toBeGreaterThan(300);
  });

  it("uses adaptive p65 from nearby samples when available", () => {
    const predicted = predictTotalDurationSeconds(
      {
        scope: "complete",
        estimated_size_bytes: 4 * 1024 * 1024 * 1024,
        recording_files: 3000,
        history_entries: 200_000,
      },
      [
        {
          scope: "complete",
          estimated_size_bytes: 4.1 * 1024 * 1024 * 1024,
          recording_files: 3100,
          history_entries: 200_000,
          duration_seconds: 500,
          completed_at_ms: Date.now() - 1000,
        },
        {
          scope: "complete",
          estimated_size_bytes: 3.9 * 1024 * 1024 * 1024,
          recording_files: 2900,
          history_entries: 210_000,
          duration_seconds: 650,
          completed_at_ms: Date.now() - 2000,
        },
        {
          scope: "complete",
          estimated_size_bytes: 4.2 * 1024 * 1024 * 1024,
          recording_files: 3050,
          history_entries: 205_000,
          duration_seconds: 900,
          completed_at_ms: Date.now() - 3000,
        },
      ],
    );
    expect(predicted).toBeGreaterThanOrEqual(650);
  });
});

describe("etaBucketKeyForRemainingSeconds", () => {
  it("maps remaining durations into coarse buckets", () => {
    expect(etaBucketKeyForRemainingSeconds(90)).toBe(
      "settings.backup.operation.etaBuckets.underTwoMinutes",
    );
    expect(etaBucketKeyForRemainingSeconds(250)).toBe(
      "settings.backup.operation.etaBuckets.twoToFiveMinutes",
    );
    expect(etaBucketKeyForRemainingSeconds(550)).toBe(
      "settings.backup.operation.etaBuckets.fiveToTenMinutes",
    );
    expect(etaBucketKeyForRemainingSeconds(800)).toBe(
      "settings.backup.operation.etaBuckets.tenToFifteenMinutes",
    );
    expect(etaBucketKeyForRemainingSeconds(1200)).toBe(
      "settings.backup.operation.etaBuckets.overFifteenMinutes",
    );
    expect(etaBucketKeyForRemainingSeconds(0)).toBe(
      "settings.backup.operation.etaBuckets.overrun",
    );
  });
});

describe("eta history persistence", () => {
  let storageMock: StorageMock;

  beforeEach(() => {
    storageMock = createStorageMock();
    vi.stubGlobal("localStorage", storageMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("persists successful samples and reloads them", () => {
    const now = Date.now();
    persistBackupEtaHistorySample({
      scope: "complete",
      estimated_size_bytes: 1024,
      recording_files: 10,
      history_entries: 50,
      duration_seconds: 500,
      completed_at_ms: now,
    });

    const loaded = loadBackupEtaHistory();
    expect(loaded).toHaveLength(1);
    expect(loaded[0].duration_seconds).toBe(500);
    expect(storageMock.getItem(BACKUP_ETA_HISTORY_STORAGE_KEY)).not.toBeNull();
  });
});

describe("isBackupRestoreCancelledError", () => {
  it("detects backend safe-cancel errors", () => {
    expect(
      isBackupRestoreCancelledError("Backup/restore was cancelled safely."),
    ).toBe(true);
  });

  it("does not classify generic failures as cancellation", () => {
    expect(
      isBackupRestoreCancelledError("Failed to create backup archive."),
    ).toBe(false);
  });
});
