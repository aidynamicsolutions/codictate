export type WorkingOperation =
  | "backup"
  | "restore-preflight"
  | "restore-apply"
  | "undo"
  | null;

export function isCreateBackupActionLoading(
  workingOperation: WorkingOperation,
): boolean {
  return workingOperation === "backup";
}

export function isRestoreBackupActionLoading(
  workingOperation: WorkingOperation,
): boolean {
  return workingOperation === "restore-preflight";
}

export function isRestoreConfirmActionLoading(
  workingOperation: WorkingOperation,
): boolean {
  return workingOperation === "restore-apply";
}

export function isUndoRestoreActionLoading(
  workingOperation: WorkingOperation,
): boolean {
  return workingOperation === "undo";
}

export function shouldShowCancelOperation(
  workingOperation: WorkingOperation,
  hasProgress: boolean,
): boolean {
  if (!hasProgress) {
    return false;
  }

  return workingOperation === "backup" || workingOperation === "restore-apply";
}
