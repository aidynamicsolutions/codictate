type Translator = (
  key: string,
  options?: Record<string, string | number | boolean | null | undefined>,
) => string;

interface BackupProgressDescriptor {
  operation: string;
  phase: string;
}

const OPERATION_LABEL_KEYS: Record<string, string> = {
  backup: "settings.backup.operation.labels.backup",
  restore: "settings.backup.operation.labels.restore",
  undo: "settings.backup.operation.labels.undo",
};

const BACKUP_PHASE_LABEL_KEYS: Record<string, string> = {
  prepare: "settings.backup.operation.phases.backupPrepare",
  "export-history": "settings.backup.operation.phases.backupExportHistory",
  "export-dictionary": "settings.backup.operation.phases.backupExportDictionary",
  "export-user-store": "settings.backup.operation.phases.backupExportUserStore",
  "export-recordings": "settings.backup.operation.phases.backupExportRecordings",
  checksums: "settings.backup.operation.phases.backupChecksums",
  package: "settings.backup.operation.phases.backupPackage",
};

const RESTORE_PHASE_LABEL_KEYS: Record<string, string> = {
  preflight: "settings.backup.operation.phases.restorePreflight",
  extract: "settings.backup.operation.phases.restoreExtract",
  "import-history": "settings.backup.operation.phases.restoreImportHistory",
  "import-dictionary": "settings.backup.operation.phases.restoreImportDictionary",
  "import-user-store": "settings.backup.operation.phases.restoreImportUserStore",
  "import-recordings": "settings.backup.operation.phases.restoreImportRecordings",
  swap: "settings.backup.operation.phases.restoreSwap",
  finalize: "settings.backup.operation.phases.restoreFinalize",
};

const UNDO_PHASE_LABEL_KEYS: Record<string, string> = {
  prepare: "settings.backup.operation.phases.undoPrepare",
  "stage-checkpoint": "settings.backup.operation.phases.undoStageCheckpoint",
  "snapshot-current": "settings.backup.operation.phases.undoSnapshotCurrent",
  swap: "settings.backup.operation.phases.undoSwap",
  cleanup: "settings.backup.operation.phases.undoCleanup",
  finalize: "settings.backup.operation.phases.undoFinalize",
};

const UNKNOWN_OPERATION_LABEL_KEY = "settings.backup.operation.labels.unknown";
const UNKNOWN_PHASE_LABEL_KEY = "settings.backup.operation.phases.unknown";

function resolveOperationLabelKey(operation: string): string {
  return OPERATION_LABEL_KEYS[operation] ?? UNKNOWN_OPERATION_LABEL_KEY;
}

function resolvePhaseLabelKey(operation: string, phase: string): string {
  if (operation === "backup") {
    return BACKUP_PHASE_LABEL_KEYS[phase] ?? UNKNOWN_PHASE_LABEL_KEY;
  }

  if (operation === "restore") {
    return RESTORE_PHASE_LABEL_KEYS[phase] ?? UNKNOWN_PHASE_LABEL_KEY;
  }

  if (operation === "undo") {
    return UNDO_PHASE_LABEL_KEYS[phase] ?? UNKNOWN_PHASE_LABEL_KEY;
  }

  return UNKNOWN_PHASE_LABEL_KEY;
}

export function formatBackupProgressLabel(
  progress: BackupProgressDescriptor,
  t: Translator,
): string {
  return t("settings.backup.operation.phase", {
    operation: t(resolveOperationLabelKey(progress.operation)),
    phase: t(resolvePhaseLabelKey(progress.operation, progress.phase)),
  });
}
