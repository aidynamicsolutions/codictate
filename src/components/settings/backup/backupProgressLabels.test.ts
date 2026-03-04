import { describe, expect, it } from "vitest";

import { formatBackupProgressLabel } from "./backupProgressLabels";

const translations: Record<string, string> = {
  "settings.backup.operation.phase": "{{operation}}: {{phase}}",
  "settings.backup.operation.labels.backup": "Backup",
  "settings.backup.operation.labels.restore": "Restore",
  "settings.backup.operation.labels.undo": "Undo",
  "settings.backup.operation.labels.unknown": "Operation",
  "settings.backup.operation.phases.unknown": "Working",
  "settings.backup.operation.phases.backupPrepare": "Preparing backup",
  "settings.backup.operation.phases.restoreImportUserStore":
    "Importing user profile data",
  "settings.backup.operation.phases.undoStageCheckpoint":
    "Staging undo checkpoint",
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

describe("formatBackupProgressLabel", () => {
  it("maps known backup phases to localized labels", () => {
    const t = makeTranslator();

    const label = formatBackupProgressLabel(
      { operation: "backup", phase: "prepare" },
      t,
    );

    expect(label).toBe("Backup: Preparing backup");
  });

  it("maps known restore phases to localized labels", () => {
    const t = makeTranslator();

    const label = formatBackupProgressLabel(
      { operation: "restore", phase: "import-user-store" },
      t,
    );

    expect(label).toBe("Restore: Importing user profile data");
  });

  it("falls back to unknown labels for unmapped tokens", () => {
    const t = makeTranslator();

    const label = formatBackupProgressLabel(
      { operation: "migrate", phase: "custom-phase" },
      t,
    );

    expect(label).toBe("Operation: Working");
  });

  it("maps known undo phases to localized labels", () => {
    const t = makeTranslator();

    const label = formatBackupProgressLabel(
      { operation: "undo", phase: "stage-checkpoint" },
      t,
    );

    expect(label).toBe("Undo: Staging undo checkpoint");
  });
});
