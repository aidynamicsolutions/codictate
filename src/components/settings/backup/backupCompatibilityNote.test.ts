import { describe, expect, it } from "vitest";

import { resolvePreflightCompatibilityNote } from "./backupCompatibilityNote";

const translations: Record<string, string> = {
  "settings.backup.restore.compatibilityNote":
    "V1 restore compatibility is guaranteed for macOS backups. Cross-platform restore is best-effort in v1.",
  "settings.backup.restore.compatibilityNotes.v1MacosGuaranteedCrossPlatformBestEffort":
    "V1 restore compatibility is guaranteed for macOS backups. Cross-platform restore is best-effort in v1.",
};

function makeTranslator() {
  return (
    key: string,
    options?: Record<string, string | number | boolean | null | undefined>,
  ): string => {
    const template = translations[key] ?? key;
    return template.replace(/\{\{(\w+)\}\}/g, (_, token: string) => {
      return String(options?.[token] ?? "");
    });
  };
}

describe("resolvePreflightCompatibilityNote", () => {
  it("maps known compatibility note codes to localized labels", () => {
    const t = makeTranslator();
    const label = resolvePreflightCompatibilityNote(
      {
        compatibility_note_code:
          "v1_macos_guaranteed_cross_platform_best_effort",
        compatibility_note: "backend fallback text",
      },
      t,
    );

    expect(label).toBe(
      "V1 restore compatibility is guaranteed for macOS backups. Cross-platform restore is best-effort in v1.",
    );
  });

  it("falls back to backend compatibility text for unknown codes", () => {
    const t = makeTranslator();
    const label = resolvePreflightCompatibilityNote(
      {
        compatibility_note_code:
          "unknown_code" as unknown as "v1_macos_guaranteed_cross_platform_best_effort",
        compatibility_note: "backend fallback text",
      },
      t,
    );

    expect(label).toBe("backend fallback text");
  });

  it("uses default localized fallback when unknown code and empty backend text", () => {
    const t = makeTranslator();
    const label = resolvePreflightCompatibilityNote(
      {
        compatibility_note_code:
          "unknown_code" as unknown as "v1_macos_guaranteed_cross_platform_best_effort",
        compatibility_note: "  ",
      },
      t,
    );

    expect(label).toBe(
      "V1 restore compatibility is guaranteed for macOS backups. Cross-platform restore is best-effort in v1.",
    );
  });
});
