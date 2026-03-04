import { describe, expect, it } from "vitest";

import {
  buildUndoRestoreStartState,
  buildRestoreApplyStartState,
  deriveRestoreImpactState,
  formatFriendlyDateTime,
  formatPreflightCreatedAt,
  formatUndoExpiresAt,
  resolvePreflightCompatibilityNoteForUi,
  shouldDisplayRecoverableFinding,
} from "./backupRestorePreflightUi";

describe("backupRestorePreflightUi", () => {
  const t = (
    key: string,
    options?: Record<string, string | number | boolean | null | undefined>,
  ): string => {
    const templates: Record<string, string> = {
      "settings.backup.restore.compatibilityNote": "Default compatibility note",
      "settings.backup.restore.compatibilityNotes.v1MacosGuaranteedCrossPlatformBestEffort":
        "Known compatibility note",
    };
    const template = templates[key] ?? key;
    return template.replace(/\{\{(\w+)\}\}/g, (_, token: string) => {
      return String(options?.[token] ?? "");
    });
  };

  it("formats friendly timestamps with a shared formatter", () => {
    const formatted = formatFriendlyDateTime("2026-03-04T02:51:29.198840+00:00", "en-US");

    expect(formatted).not.toBe("2026-03-04T02:51:29.198840+00:00");
    expect(formatted).toMatch(/2026/);
  });

  it("formats RFC3339 preflight timestamps into a localized friendly string", () => {
    const formatted = formatPreflightCreatedAt("2026-03-04T02:51:29.198840+00:00", "en-US");

    expect(formatted).not.toBe("2026-03-04T02:51:29.198840+00:00");
    expect(formatted).toMatch(/2026/);
  });

  it("falls back to original created_at value when parsing fails", () => {
    const raw = "not-a-date";
    expect(formatPreflightCreatedAt(raw, "en-US")).toBe(raw);
  });

  it("formats undo expiration dates with the same fallback behavior", () => {
    const raw = "2026-03-11T10:50:58.515524+00:00";
    const formatted = formatUndoExpiresAt(raw, "en-US");

    expect(formatted).not.toBe(raw);
    expect(formatUndoExpiresAt("still-not-a-date", "en-US")).toBe("still-not-a-date");
  });

  it("hides recoverable findings that are technical and low-value for users", () => {
    expect(
      shouldDisplayRecoverableFinding({ code: "archive_extension_unexpected" }),
    ).toBe(false);
    expect(shouldDisplayRecoverableFinding({ code: "cross_platform_best_effort" })).toBe(
      true,
    );
  });

  it("returns a localized compatibility note for known compatibility codes", () => {
    expect(
      resolvePreflightCompatibilityNoteForUi(
        {
          compatibility_note_code:
            "v1_macos_guaranteed_cross_platform_best_effort",
          compatibility_note: "backend fallback text",
        },
        t,
      ),
    ).toBe("Known compatibility note");
  });

  it("returns null compatibility copy when preflight report is absent", () => {
    expect(resolvePreflightCompatibilityNoteForUi(null, t)).toBeNull();
  });

  it("starts restore apply by closing preflight dialog and seeding restore progress", () => {
    expect(buildRestoreApplyStartState()).toEqual({
      showPreflightDialog: false,
      workingOperation: "restore-apply",
      progress: {
        operation: "restore",
        phase: "preflight",
        current: 1,
        total: 8,
      },
    });
  });

  it("starts undo restore by seeding undo progress immediately", () => {
    expect(buildUndoRestoreStartState()).toEqual({
      workingOperation: "undo",
      progress: {
        operation: "undo",
        phase: "prepare",
        current: 50,
        total: 1000,
      },
    });
  });

  it("derives fresh-install impact state from local backup estimate counts", () => {
    expect(
      deriveRestoreImpactState(
        {
          history_entries: 0,
          dictionary_entries: 0,
          recording_files: 0,
        },
        {
          includes_recordings: true,
        },
      ),
    ).toEqual({
      hasLocalSnapshot: true,
      isFreshInstall: true,
      willRemoveRecordings: false,
      localRecordingFiles: 0,
    });
  });

  it("flags recording removal risk for no-recordings backup over existing recordings", () => {
    expect(
      deriveRestoreImpactState(
        {
          history_entries: 23,
          dictionary_entries: 4,
          recording_files: 11,
        },
        {
          includes_recordings: false,
        },
      ),
    ).toEqual({
      hasLocalSnapshot: true,
      isFreshInstall: false,
      willRemoveRecordings: true,
      localRecordingFiles: 11,
    });
  });

  it("returns conservative fallback impact when local estimate is unavailable", () => {
    expect(
      deriveRestoreImpactState(null, {
        includes_recordings: false,
      }),
    ).toEqual({
      hasLocalSnapshot: false,
      isFreshInstall: false,
      willRemoveRecordings: false,
      localRecordingFiles: 0,
    });
  });
});
