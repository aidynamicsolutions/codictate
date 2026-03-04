import type {
  BackupEstimateReport,
  PreflightRestoreReport,
  PreflightSummary,
  RestoreFinding,
} from "@/bindings";
import { resolvePreflightCompatibilityNote } from "./backupCompatibilityNote";
import type { WorkingOperation } from "./backupOperationUiState";

type BackupProgressOperation = "restore" | "undo";
type BackupProgressPhase = "preflight" | "prepare";
type Translator = (
  key: string,
  options?: Record<string, string | number | boolean | null | undefined>,
) => string;

export interface BackupProgressSeed {
  operation: BackupProgressOperation;
  phase: BackupProgressPhase;
  current: number;
  total: number;
}

export interface RestoreApplyStartState {
  showPreflightDialog: boolean;
  workingOperation: WorkingOperation;
  progress: BackupProgressSeed;
}

export interface UndoRestoreStartState {
  workingOperation: WorkingOperation;
  progress: BackupProgressSeed;
}

export interface RestoreImpactState {
  hasLocalSnapshot: boolean;
  isFreshInstall: boolean;
  willRemoveRecordings: boolean;
  localRecordingFiles: number;
}

const HIDDEN_RECOVERABLE_FINDING_CODES = new Set(["archive_extension_unexpected"]);

export function shouldDisplayRecoverableFinding(
  finding: Pick<RestoreFinding, "code">,
): boolean {
  return !HIDDEN_RECOVERABLE_FINDING_CODES.has(finding.code);
}

export function formatFriendlyDateTime(value: string, locale: string): string {
  if (value.trim().length === 0) {
    return value;
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  try {
    return new Intl.DateTimeFormat(locale, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(parsed);
  } catch {
    return value;
  }
}

export function formatPreflightCreatedAt(createdAt: string, locale: string): string {
  return formatFriendlyDateTime(createdAt, locale);
}

export function formatUndoExpiresAt(expiresAt: string, locale: string): string {
  return formatFriendlyDateTime(expiresAt, locale);
}

export function resolvePreflightCompatibilityNoteForUi(
  report: Pick<
    PreflightRestoreReport,
    "compatibility_note_code" | "compatibility_note"
  > | null,
  t: Translator,
): string | null {
  if (!report) {
    return null;
  }

  return resolvePreflightCompatibilityNote(report, t);
}

export function deriveRestoreImpactState(
  localEstimate: Pick<
    BackupEstimateReport,
    "history_entries" | "dictionary_entries" | "recording_files"
  > | null,
  preflightSummary: Pick<PreflightSummary, "includes_recordings"> | null,
): RestoreImpactState {
  if (!localEstimate) {
    return {
      hasLocalSnapshot: false,
      isFreshInstall: false,
      willRemoveRecordings: false,
      localRecordingFiles: 0,
    };
  }

  const localRecordingFiles = Math.max(0, localEstimate.recording_files);
  const isFreshInstall =
    localEstimate.history_entries === 0 &&
    localEstimate.dictionary_entries === 0 &&
    localRecordingFiles === 0;
  const willRemoveRecordings =
    preflightSummary != null &&
    !preflightSummary.includes_recordings &&
    localRecordingFiles > 0;

  return {
    hasLocalSnapshot: true,
    isFreshInstall,
    willRemoveRecordings,
    localRecordingFiles,
  };
}

export function buildRestoreApplyStartState(): RestoreApplyStartState {
  return {
    showPreflightDialog: false,
    workingOperation: "restore-apply",
    progress: {
      operation: "restore",
      phase: "preflight",
      current: 1,
      total: 8,
    },
  };
}

export function buildUndoRestoreStartState(): UndoRestoreStartState {
  return {
    workingOperation: "undo",
    progress: {
      operation: "undo",
      phase: "prepare",
      current: 50,
      total: 1000,
    },
  };
}
