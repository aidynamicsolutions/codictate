import { describe, expect, it } from "vitest";

import {
  isCreateBackupActionLoading,
  isRestoreBackupActionLoading,
  isRestoreConfirmActionLoading,
  isUndoRestoreActionLoading,
  shouldShowCancelOperation,
  type WorkingOperation,
} from "./backupOperationUiState";

const OPERATIONS: WorkingOperation[] = [
  "backup",
  "restore-preflight",
  "restore-apply",
  "undo",
  null,
];

describe("backupOperationUiState", () => {
  it("shows create button loading only for backup operations", () => {
    const loadingOperations = OPERATIONS.filter((operation) =>
      isCreateBackupActionLoading(operation),
    );

    expect(loadingOperations).toEqual(["backup"]);
  });

  it("shows restore button loading only for restore preflight operations", () => {
    const loadingOperations = OPERATIONS.filter((operation) =>
      isRestoreBackupActionLoading(operation),
    );

    expect(loadingOperations).toEqual(["restore-preflight"]);
  });

  it("shows restore confirm loading only for restore apply operations", () => {
    const loadingOperations = OPERATIONS.filter((operation) =>
      isRestoreConfirmActionLoading(operation),
    );

    expect(loadingOperations).toEqual(["restore-apply"]);
  });

  it("shows undo loading only for undo operations", () => {
    const loadingOperations = OPERATIONS.filter((operation) =>
      isUndoRestoreActionLoading(operation),
    );

    expect(loadingOperations).toEqual(["undo"]);
  });

  it("hides cancel during restore preflight", () => {
    expect(shouldShowCancelOperation("restore-preflight", true)).toBe(false);
  });

  it("shows cancel only for backup and restore apply while progress exists", () => {
    expect(shouldShowCancelOperation("backup", true)).toBe(true);
    expect(shouldShowCancelOperation("restore-apply", true)).toBe(true);
    expect(shouldShowCancelOperation("undo", true)).toBe(false);
    expect(shouldShowCancelOperation("backup", false)).toBe(false);
  });
});
