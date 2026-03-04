import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Check, ChevronDown, Loader2, RotateCcw, Save, Upload } from "lucide-react";

import {
  commands,
  type BackupEstimateReport,
  type BackupScope,
  type PreflightRestoreReport,
  type RestoreFinding,
  type UndoLastRestoreAvailabilityReport,
} from "@/bindings";
import { Button } from "@/components/shared/ui/button";
import { Card, CardContent } from "@/components/shared/ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/shared/ui/collapsible";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { Progress } from "@/components/shared/ui/progress";
import { Switch } from "@/components/shared/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { logDebug, logError, logInfo } from "@/utils/logging";
import {
  buildSuggestedBackupFileName,
  buildBackupSaveDefaultPath,
  getBackupOpenDefaultPath,
  rememberBackupDirectoryFromFilePath,
} from "./backupDialogPathing";
import { selectBackupArchivePath, selectBackupSavePath } from "./backupDialogGuards";
import {
  etaBucketKeyForRemainingSeconds,
  isBackupRestoreCancelledError,
  loadBackupEtaHistory,
  persistBackupEtaHistorySample,
  predictTotalDurationSeconds,
  type BackupEtaFeatures,
} from "./backupProgressModel";
import { formatBackupProgressLabel } from "./backupProgressLabels";
import { showBackupSuccessToastSequence } from "./backupToastSequencing";
import {
  isCreateBackupActionLoading,
  isRestoreBackupActionLoading,
  isRestoreConfirmActionLoading,
  isUndoRestoreActionLoading,
  shouldShowCancelOperation,
  type WorkingOperation,
} from "./backupOperationUiState";
import {
  buildUndoRestoreStartState,
  buildRestoreApplyStartState,
  deriveRestoreImpactState,
  formatPreflightCreatedAt,
  formatUndoExpiresAt,
  resolvePreflightCompatibilityNoteForUi,
  shouldDisplayRecoverableFinding,
} from "./backupRestorePreflightUi";

interface BackupProgressPayload {
  operation: string;
  phase: string;
  current: number;
  total: number;
}

const BACKUP_ETA_BUCKET_UPDATE_MS = 5000;
const BACKUP_ETA_OVERRUN_KEY = "settings.backup.operation.etaBuckets.overrun";

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / Math.pow(1024, exponent);
  const precision = value >= 10 || exponent === 0 ? 0 : 1;
  return `${value.toFixed(precision)} ${units[exponent]}`;
}

export function BackupRestoreCard() {
  const { t, i18n } = useTranslation();

  const [workingOperation, setWorkingOperation] =
    useState<WorkingOperation>(null);
  const [isEstimating, setIsEstimating] = useState(false);
  const [progress, setProgress] = useState<BackupProgressPayload | null>(null);
  const [preflightReport, setPreflightReport] =
    useState<PreflightRestoreReport | null>(null);
  const [pendingRestorePath, setPendingRestorePath] = useState<string | null>(
    null,
  );
  const [undoAvailability, setUndoAvailability] =
    useState<UndoLastRestoreAvailabilityReport | null>(null);
  const [backupEstimate, setBackupEstimate] =
    useState<BackupEstimateReport | null>(null);
  const [preflightLocalEstimate, setPreflightLocalEstimate] =
    useState<BackupEstimateReport | null>(null);
  const [includeRecordings, setIncludeRecordings] = useState(true);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showPreflightDialog, setShowPreflightDialog] = useState(false);
  const [isBackupDetailsOpen, setIsBackupDetailsOpen] = useState(false);
  const [backupStartedAtMs, setBackupStartedAtMs] = useState<number | null>(
    null,
  );
  const [backupEtaBucketKey, setBackupEtaBucketKey] = useState<string | null>(
    null,
  );
  const [backupEtaTick, setBackupEtaTick] = useState(0);
  const backupStartedAtMsRef = useRef<number | null>(null);
  const backupEtaPredictedTotalSecondsRef = useRef<number | null>(null);
  const backupEtaFeaturesRef = useRef<BackupEtaFeatures | null>(null);
  const backupEtaLastBucketKeyRef = useRef<string | null>(null);
  const backupEtaLastBucketUpdatedAtMsRef = useRef<number | null>(null);
  const backupEtaSessionIdRef = useRef(0);
  const isWorking = workingOperation !== null;
  const isCreateLoading = isCreateBackupActionLoading(workingOperation);
  const isRestorePreflightLoading =
    isRestoreBackupActionLoading(workingOperation);
  const isRestoreApplyLoading =
    isRestoreConfirmActionLoading(workingOperation);
  const isUndoLoading = isUndoRestoreActionLoading(workingOperation);
  const showOperationCancel = shouldShowCancelOperation(
    workingOperation,
    progress != null,
  );

  const formatFindingMessage = useCallback(
    (finding: RestoreFinding, kind: "blocking" | "recoverable") => {
      if (kind === "recoverable") {
        if (finding.code === "cross_platform_best_effort") {
          return t("settings.backup.restore.findings.recoverableCrossPlatform");
        }
        if (
          finding.code === "user_store_payload_recoverable" ||
          finding.code === "user_store_missing_recoverable"
        ) {
          return t("settings.backup.restore.findings.recoverableUserStore");
        }
        if (finding.code === "archive_extension_unexpected") {
          return t("settings.backup.restore.findings.recoverableExtension");
        }
        if (
          finding.code === "user_stats_missing_recoverable" ||
          finding.code === "user_stats_payload_recoverable"
        ) {
          return t("settings.backup.restore.findings.recoverableStatsRecomputed");
        }
        return t("settings.backup.restore.findings.genericRecoverable");
      }

      if (finding.code === "local_history_data_corrupted") {
        return t("settings.backup.restore.findings.blockingLocalHistoryCorrupted");
      }
      if (
        finding.code === "insufficient_free_space" ||
        finding.code === "available_space_check_failed"
      ) {
        return t("settings.backup.restore.findings.blockingDiskSpace");
      }
      if (
        finding.code.startsWith("checksum_") ||
        finding.code === "checksum_mismatch"
      ) {
        return t("settings.backup.restore.findings.blockingChecksum");
      }
      if (finding.code.startsWith("history_")) {
        return t("settings.backup.restore.findings.blockingHistoryPayload");
      }
      if (finding.code === "dictionary_payload_invalid") {
        return t("settings.backup.restore.findings.blockingDictionaryPayload");
      }
      if (
        finding.code.startsWith("archive_") ||
        finding.code === "missing_required_payload" ||
        finding.code === "required_payload_size_limit" ||
        finding.code === "manifest_parse_failed" ||
        finding.code === "backup_format_version_unsupported"
      ) {
        return t("settings.backup.restore.findings.blockingArchive");
      }

      return t("settings.backup.restore.findings.genericBlocking");
    },
    [t],
  );

  const rawProgressRatio = useMemo(() => {
    if (!progress || progress.total <= 0) return 0;
    return Math.max(0, Math.min(1, progress.current / progress.total));
  }, [progress]);

  const progressPercent = useMemo(() => {
    return Math.max(0, Math.min(100, Math.round(rawProgressRatio * 100)));
  }, [rawProgressRatio]);

  const backupEtaLabel = useMemo(() => {
    if (!progress || progress.operation !== "backup" || !backupEtaBucketKey) {
      return null;
    }
    return t(backupEtaBucketKey);
  }, [backupEtaBucketKey, progress, t]);

  const selectedScope: BackupScope = includeRecordings ? "complete" : "smaller";

  const selectedEstimatedBytes = useMemo(() => {
    if (!backupEstimate) return null;
    return includeRecordings
      ? backupEstimate.complete_estimated_size_bytes
      : backupEstimate.smaller_estimated_size_bytes;
  }, [backupEstimate, includeRecordings]);

  const visibleRecoverableFindings = useMemo(() => {
    if (!preflightReport?.recoverable_findings) {
      return [];
    }
    return preflightReport.recoverable_findings.filter((finding) =>
      shouldDisplayRecoverableFinding(finding),
    );
  }, [preflightReport?.recoverable_findings]);

  const formattedPreflightCreatedAt = useMemo(() => {
    if (!preflightReport?.summary?.created_at) {
      return "";
    }
    return formatPreflightCreatedAt(preflightReport.summary.created_at, i18n.language);
  }, [i18n.language, preflightReport?.summary?.created_at]);

  const formattedUndoExpiresAt = useMemo(() => {
    if (!undoAvailability?.expires_at) {
      return "";
    }

    return formatUndoExpiresAt(undoAvailability.expires_at, i18n.language);
  }, [i18n.language, undoAvailability?.expires_at]);

  const preflightCompatibilityNote = useMemo(() => {
    return resolvePreflightCompatibilityNoteForUi(preflightReport, t);
  }, [preflightReport, t]);

  const restoreImpactState = useMemo(() => {
    return deriveRestoreImpactState(
      preflightLocalEstimate,
      preflightReport?.summary ?? null,
    );
  }, [preflightLocalEstimate, preflightReport?.summary]);

  const resetBackupEtaState = useCallback(() => {
    setBackupStartedAtMs(null);
    setBackupEtaBucketKey(null);
    backupStartedAtMsRef.current = null;
    backupEtaPredictedTotalSecondsRef.current = null;
    backupEtaFeaturesRef.current = null;
    backupEtaLastBucketKeyRef.current = null;
    backupEtaLastBucketUpdatedAtMsRef.current = null;
    backupEtaSessionIdRef.current += 1;
  }, []);

  const buildBackupEtaFeatures = useCallback(
    (
      scope: BackupScope,
      estimate: BackupEstimateReport,
    ): BackupEtaFeatures => {
      return {
        scope,
        estimated_size_bytes:
          scope === "complete"
            ? estimate.complete_estimated_size_bytes
            : estimate.smaller_estimated_size_bytes,
        recording_files: scope === "complete" ? estimate.recording_files : 0,
        history_entries: estimate.history_entries,
      };
    },
    [],
  );

  const ensureBackupEstimate = useCallback(async (): Promise<BackupEstimateReport | null> => {
    if (backupEstimate) {
      return backupEstimate;
    }

    try {
      const result = await commands.getBackupEstimate();
      if (result.status === "ok") {
        setBackupEstimate(result.data);
        return result.data;
      }
      logError(
        `Failed to estimate backup size for ETA: ${result.error}`,
        "fe-backup-restore",
      );
      return null;
    } catch (error) {
      logError(`Failed to estimate backup size for ETA: ${error}`, "fe-backup-restore");
      return null;
    }
  }, [backupEstimate]);

  const initializeBackupEta = useCallback(
    async (scope: BackupScope, sessionId: number) => {
      try {
        const estimate = await ensureBackupEstimate();
        if (backupEtaSessionIdRef.current !== sessionId) {
          return;
        }
        if (!estimate) {
          setBackupEtaBucketKey(null);
          backupEtaPredictedTotalSecondsRef.current = null;
          backupEtaFeaturesRef.current = null;
          backupEtaLastBucketKeyRef.current = null;
          backupEtaLastBucketUpdatedAtMsRef.current = null;
          return;
        }

        const features = buildBackupEtaFeatures(scope, estimate);
        const history = loadBackupEtaHistory();
        const predictedTotalSeconds = predictTotalDurationSeconds(features, history);
        const initialBucketKey =
          etaBucketKeyForRemainingSeconds(predictedTotalSeconds);
        const nowMs = Date.now();

        backupEtaFeaturesRef.current = features;
        backupEtaPredictedTotalSecondsRef.current = predictedTotalSeconds;
        backupEtaLastBucketKeyRef.current = initialBucketKey;
        backupEtaLastBucketUpdatedAtMsRef.current = nowMs;
        setBackupEtaBucketKey(initialBucketKey);

        logInfo(
          `ETA_INIT scope=${features.scope} estimated_size_bytes=${features.estimated_size_bytes} recording_files=${features.recording_files} history_entries=${features.history_entries} history_samples=${history.length} predicted_total_seconds=${Math.round(predictedTotalSeconds)} bucket=${initialBucketKey}`,
          "fe-backup-restore",
        );
      } catch (error) {
        logError(`Failed to initialize backup ETA model: ${error}`, "fe-backup-restore");
      }
    },
    [buildBackupEtaFeatures, ensureBackupEstimate],
  );

  const refreshUndoAvailability = useCallback(async () => {
    const fallbackAvailability: UndoLastRestoreAvailabilityReport = {
      available: false,
      expires_at: null,
      message: t("settings.backup.undo.unavailable"),
    };

    try {
      const result = await commands.getUndoLastRestoreAvailability();
      if (result.status === "ok") {
        setUndoAvailability(result.data);
        return;
      }

      logError(
        `Failed to get undo restore availability: ${result.error}`,
        "fe-backup-restore",
      );
      setUndoAvailability(fallbackAvailability);
    } catch (error) {
      logError(
        `Failed to get undo restore availability: ${error}`,
        "fe-backup-restore",
      );
      setUndoAvailability(fallbackAvailability);
    }
  }, [t]);

  const loadBackupEstimate = useCallback(async () => {
    setIsEstimating(true);
    try {
      const result = await commands.getBackupEstimate();
      if (result.status === "ok") {
        setBackupEstimate(result.data);
      } else {
        logError(
          `Failed to estimate backup size: ${result.error}`,
          "fe-backup-restore",
        );
        setBackupEstimate(null);
      }
    } catch (error) {
      logError(`Failed to estimate backup size: ${error}`, "fe-backup-restore");
      setBackupEstimate(null);
    } finally {
      setIsEstimating(false);
    }
  }, []);

  useEffect(() => {
    refreshUndoAvailability();

    const unlistenPromise = listen<BackupProgressPayload>(
      "backup-progress",
      (event) => {
        setProgress(event.payload);
      },
    );

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [refreshUndoAvailability]);

  useEffect(() => {
    if (!showCreateDialog) {
      return;
    }
    loadBackupEstimate();
  }, [loadBackupEstimate, showCreateDialog]);

  useEffect(() => {
    if (!isWorking || progress?.operation !== "backup") {
      return;
    }

    const intervalId = window.setInterval(() => {
      setBackupEtaTick((value) => value + 1);
    }, 1000);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [isWorking, progress?.operation]);

  useEffect(() => {
    if (
      !isWorking ||
      !progress ||
      progress.operation !== "backup" ||
      backupStartedAtMs == null
    ) {
      setBackupEtaBucketKey(null);
      return;
    }

    const predictedTotalSeconds = backupEtaPredictedTotalSecondsRef.current;
    if (predictedTotalSeconds == null) {
      setBackupEtaBucketKey(null);
      return;
    }

    const nowMs = Date.now();
    const elapsedSeconds = Math.max(0, (nowMs - backupStartedAtMs) / 1000);
    const remainingSeconds = predictedTotalSeconds - elapsedSeconds;
    const nextBucketKey = etaBucketKeyForRemainingSeconds(remainingSeconds);
    const previousBucketKey = backupEtaLastBucketKeyRef.current;
    const lastUpdatedAtMs = backupEtaLastBucketUpdatedAtMsRef.current;

    if (
      nextBucketKey === previousBucketKey &&
      lastUpdatedAtMs != null &&
      nowMs - lastUpdatedAtMs < BACKUP_ETA_BUCKET_UPDATE_MS
    ) {
      return;
    }

    if (previousBucketKey !== nextBucketKey) {
      logDebug(
        `ETA_BUCKET_CHANGE previous=${previousBucketKey ?? "none"} next=${nextBucketKey} elapsed_seconds=${Math.round(elapsedSeconds)} remaining_seconds=${Math.round(remainingSeconds)}`,
        "fe-backup-restore",
      );
    }

    if (
      nextBucketKey === BACKUP_ETA_OVERRUN_KEY &&
      previousBucketKey !== BACKUP_ETA_OVERRUN_KEY
    ) {
      logInfo(
        `ETA_OVERRUN elapsed_seconds=${Math.round(elapsedSeconds)} predicted_total_seconds=${Math.round(predictedTotalSeconds)}`,
        "fe-backup-restore",
      );
    }

    backupEtaLastBucketKeyRef.current = nextBucketKey;
    backupEtaLastBucketUpdatedAtMsRef.current = nowMs;
    setBackupEtaBucketKey(nextBucketKey);
  }, [backupEtaTick, backupStartedAtMs, isWorking, progress]);

  const runBackup = useCallback(
    async (scope: BackupScope) => {
      const suggestedFileName = buildSuggestedBackupFileName(scope);

      const outputPath = await selectBackupSavePath(
        () =>
          save({
            title: t("settings.backup.export.selectDestination"),
            defaultPath: buildBackupSaveDefaultPath(suggestedFileName),
            filters: [
              {
                name: t("settings.backup.export.fileType"),
                extensions: ["codictatebackup"],
              },
            ],
          }),
        (error) => {
          logError(`Backup save dialog failed: ${error}`, "fe-backup-restore");
          toast.error(t("settings.backup.export.errorTitle"), {
            description: t("settings.backup.export.errorDescription"),
          });
        },
      );

      if (!outputPath) {
        return;
      }

      rememberBackupDirectoryFromFilePath(outputPath);

      const startedAtMs = Date.now();
      setWorkingOperation("backup");
      setProgress(null);
      setBackupStartedAtMs(startedAtMs);
      backupStartedAtMsRef.current = startedAtMs;
      setBackupEtaTick((value) => value + 1);
      backupEtaSessionIdRef.current += 1;
      const etaSessionId = backupEtaSessionIdRef.current;
      void initializeBackupEta(scope, etaSessionId);
      try {
        const result = await commands.createBackup({
          scope,
          output_path: outputPath,
        });

        if (result.status === "error") {
          throw new Error(result.error);
        }

        const warningSuffix =
          result.data.warnings.length > 0
            ? ` ${t("settings.backup.export.withWarnings", {
                count: result.data.warnings.length,
              })}`
            : "";

        showBackupSuccessToastSequence({
          successTitle: t("settings.backup.export.successTitle"),
          successDescription:
            t("settings.backup.export.successDescription", {
              path: result.data.output_path,
            }) + warningSuffix,
          unencryptedInfo: t("settings.backup.export.unencryptedInfo"),
        });

        const features = backupEtaFeaturesRef.current;
        const startedAt = backupStartedAtMsRef.current;
        const predictedTotalSeconds = backupEtaPredictedTotalSecondsRef.current;
        if (features && startedAt != null) {
          const completedAtMs = Date.now();
          const actualDurationSeconds = Math.max(
            1,
            Math.round((completedAtMs - startedAt) / 1000),
          );
          persistBackupEtaHistorySample({
            ...features,
            duration_seconds: actualDurationSeconds,
            completed_at_ms: completedAtMs,
          });
          logInfo(
            `ETA_ACTUAL scope=${features.scope} estimated_size_bytes=${features.estimated_size_bytes} recording_files=${features.recording_files} history_entries=${features.history_entries} predicted_total_seconds=${predictedTotalSeconds != null ? Math.round(predictedTotalSeconds) : "unknown"} actual_seconds=${actualDurationSeconds}`,
            "fe-backup-restore",
          );
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (isBackupRestoreCancelledError(message)) {
          logInfo(
            `Backup export cancelled by user: ${message}`,
            "fe-backup-restore",
          );
        } else {
          logError(`Backup export failed: ${error}`, "fe-backup-restore");
          const description = message.includes("export_payload_size_limit:")
            ? t("settings.backup.export.payloadSizeLimitDescription")
            : t("settings.backup.export.errorDescription");
          toast.error(t("settings.backup.export.errorTitle"), {
            description,
          });
        }
      } finally {
        setWorkingOperation(null);
        setProgress(null);
        resetBackupEtaState();
      }
    },
    [initializeBackupEta, resetBackupEtaState, t],
  );

  const runPreflight = useCallback(async () => {
    const archivePath = await selectBackupArchivePath(
      () =>
        open({
          title: t("settings.backup.restore.selectBackup"),
          directory: false,
          multiple: false,
          defaultPath: getBackupOpenDefaultPath(),
          filters: [
            {
              name: t("settings.backup.restore.fileType"),
              extensions: ["codictatebackup"],
            },
          ],
        }),
      (error) => {
        logError(`Restore open dialog failed: ${error}`, "fe-backup-restore");
        toast.error(t("settings.backup.restore.preflightErrorTitle"), {
          description: t("settings.backup.restore.preflightErrorDescription"),
        });
      },
    );

    if (!archivePath) {
      return;
    }

    rememberBackupDirectoryFromFilePath(archivePath);
    setPreflightLocalEstimate(null);
    const localEstimateSnapshotPromise = ensureBackupEstimate();

    setWorkingOperation("restore-preflight");
    setProgress({
      operation: "restore",
      phase: "preflight",
      current: 0,
      total: 1000,
    });
    resetBackupEtaState();

    try {
      const result = await commands.preflightRestore({
        archive_path: archivePath,
      });

      if (result.status === "error") {
        throw new Error(result.error);
      }

      const localEstimateSnapshot = await localEstimateSnapshotPromise;
      setPreflightLocalEstimate(localEstimateSnapshot);
      setPendingRestorePath(archivePath);
      setPreflightReport(result.data);
      setIsBackupDetailsOpen(false);
      setShowPreflightDialog(true);
    } catch (error) {
      logError(`Restore preflight failed: ${error}`, "fe-backup-restore");
      toast.error(t("settings.backup.restore.preflightErrorTitle"), {
        description: t("settings.backup.restore.preflightErrorDescription"),
      });
    } finally {
      setWorkingOperation(null);
      setProgress(null);
    }
  }, [ensureBackupEstimate, resetBackupEtaState, t]);

  const runRestore = useCallback(async () => {
    if (!pendingRestorePath || !preflightReport?.can_apply) {
      return;
    }

    const restoreStartState = buildRestoreApplyStartState();
    setShowPreflightDialog(restoreStartState.showPreflightDialog);
    setWorkingOperation(restoreStartState.workingOperation);
    setProgress(restoreStartState.progress);
    resetBackupEtaState();

    try {
      const result = await commands.applyRestore({
        archive_path: pendingRestorePath,
      });

      if (result.status === "error") {
        throw new Error(result.error);
      }

      if (result.data.warnings.length > 0) {
        toast.warning(t("settings.backup.restore.successWithWarningsTitle"), {
          description: t(
            "settings.backup.restore.successWithWarningsDescription",
            {
              count: result.data.warnings.length,
            },
          ),
        });
      } else {
        toast.success(t("settings.backup.restore.successTitle"), {
          description: t("settings.backup.restore.successDescription"),
        });
      }

      await refreshUndoAvailability();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (isBackupRestoreCancelledError(message)) {
        logInfo(`Restore apply cancelled by user: ${message}`, "fe-backup-restore");
      } else {
        logError(`Restore apply failed: ${error}`, "fe-backup-restore");
        toast.error(t("settings.backup.restore.errorTitle"), {
          description: t("settings.backup.restore.errorDescription"),
        });
      }
    } finally {
      setWorkingOperation(null);
      setProgress(null);
      resetBackupEtaState();
    }
  }, [
    pendingRestorePath,
    preflightReport?.can_apply,
    refreshUndoAvailability,
    resetBackupEtaState,
    t,
  ]);

  const runUndoRestore = useCallback(async () => {
    const undoStartState = buildUndoRestoreStartState();
    setWorkingOperation(undoStartState.workingOperation);
    setProgress(undoStartState.progress);
    resetBackupEtaState();

    try {
      const result = await commands.undoLastRestore({});
      if (result.status === "error") {
        throw new Error(result.error);
      }

      if (result.data.restored) {
        toast.success(t("settings.backup.undo.successTitle"), {
          description: result.data.message,
        });
      } else {
        toast.message(result.data.message);
      }

      await refreshUndoAvailability();
    } catch (error) {
      logError(`Undo restore failed: ${error}`, "fe-backup-restore");
      toast.error(t("settings.backup.undo.errorTitle"), {
        description: t("settings.backup.undo.errorDescription"),
      });
    } finally {
      setWorkingOperation(null);
      setProgress(null);
      resetBackupEtaState();
    }
  }, [refreshUndoAvailability, resetBackupEtaState, t]);

  const cancelActiveOperation = useCallback(async () => {
    try {
      await commands.cancelOperation();
      toast.message(t("settings.backup.operation.cancelRequested"));
    } catch (error) {
      logError(
        `Failed to cancel backup/restore operation: ${error}`,
        "fe-backup-restore",
      );
    }
  }, [t]);

  const openCreateBackupDialog = useCallback(() => {
    setIncludeRecordings(true);
    setShowCreateDialog(true);
  }, []);

  const handleCreateBackupConfirm = useCallback(async () => {
    setShowCreateDialog(false);
    await runBackup(selectedScope);
  }, [runBackup, selectedScope]);

  return (
    <>
      <div className="w-full space-y-3">
        <div className="px-1">
          <h3 className="text-sm font-medium text-muted-foreground tracking-wide pl-1">
            {t("settings.backup.title")}
          </h3>
        </div>

        <Card className="w-full bg-card border shadow-sm rounded-xl overflow-hidden">
          <CardContent className="p-0">
            <div className="divide-y divide-border/40 px-6">
              <div className="flex items-center justify-between gap-4 py-4">
                <div className="flex flex-col gap-1.5 flex-1 mr-6">
                  <div className="text-[13px] text-muted-foreground/80 leading-relaxed font-normal">
                    {t("settings.backup.description")}
                  </div>
                </div>

                <div className="flex flex-wrap gap-2 shrink-0 justify-end">
                  <Button
                    type="button"
                    variant="secondary"
                    disabled={isWorking}
                    onClick={openCreateBackupDialog}
                    className="h-9 px-6 min-w-[11rem] text-sm font-medium shadow-sm rounded-md gap-2"
                  >
                    {isCreateLoading ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Save className="h-4 w-4" />
                    )}
                    {t("settings.backup.export.createAction")}
                  </Button>

                  <Tooltip>
                    <TooltipTrigger asChild>
                      <span className="inline-flex">
                        <Button
                          type="button"
                          variant="outline"
                          disabled={isWorking}
                          onClick={runPreflight}
                          className="h-9 px-6 min-w-[11rem] text-sm font-medium shadow-sm rounded-md gap-2"
                        >
                          {isRestorePreflightLoading ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                          ) : (
                            <Upload className="h-4 w-4" />
                          )}
                          {t("settings.backup.restore.action")}
                        </Button>
                      </span>
                    </TooltipTrigger>
                    <TooltipContent side="top" sideOffset={8} className="max-w-xs">
                      <p>{t("settings.backup.restore.selectionHint")}</p>
                    </TooltipContent>
                  </Tooltip>
                </div>
              </div>

              {progress ? (
                <div className="py-3 space-y-2">
                  <div className="flex items-center justify-between text-sm text-muted-foreground gap-4">
                    <div className="flex flex-col">
                      <span>{formatBackupProgressLabel(progress, t)}</span>
                      {backupEtaLabel ? (
                        <span className="text-xs text-muted-foreground/80">
                          {backupEtaLabel}
                        </span>
                      ) : null}
                    </div>
                    <div className="flex items-center gap-3 shrink-0">
                      <span>{progressPercent}%</span>
                      {showOperationCancel ? (
                        <Button
                          type="button"
                          variant="destructive"
                          onClick={cancelActiveOperation}
                          className="h-8 px-3 text-xs"
                        >
                          {t("settings.backup.operation.cancel")}
                        </Button>
                      ) : null}
                    </div>
                  </div>
                  <Progress value={progressPercent} />
                </div>
              ) : null}

              {undoAvailability?.available ? (
                <div className="flex items-center justify-between gap-4 py-3">
                  <div className="text-xs text-muted-foreground">
                    {t("settings.backup.undo.availableUntil", {
                      expiresAt:
                        formattedUndoExpiresAt || undoAvailability.expires_at,
                    })}
                  </div>
                  <Button
                    type="button"
                    variant="ghost"
                    disabled={isWorking}
                    onClick={runUndoRestore}
                    className="h-8 px-3 text-xs gap-2"
                  >
                    {isUndoLoading ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <RotateCcw className="h-4 w-4" />
                    )}
                    {t("settings.backup.undo.action")}
                  </Button>
                </div>
              ) : null}
            </div>
          </CardContent>
        </Card>
      </div>

      <Dialog
        open={showCreateDialog}
        onOpenChange={(open) => {
          if (!isWorking) {
            setShowCreateDialog(open);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("settings.backup.export.configureTitle")}</DialogTitle>
            <DialogDescription>
              {t("settings.backup.export.configureDescription")}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3 text-sm">
            <div className="rounded-md border p-3">
              <div className="font-medium text-foreground">
                {t("settings.backup.export.includesTitle")}
              </div>
              <div className="mt-2 space-y-2 text-muted-foreground">
                <div className="flex items-center gap-2">
                  <Check className="h-4 w-4 text-emerald-600" />
                  <span>{t("settings.backup.export.includesHistory")}</span>
                </div>
                <div className="flex items-center gap-2">
                  <Check className="h-4 w-4 text-emerald-600" />
                  <span>{t("settings.backup.export.includesDictionary")}</span>
                </div>
                <div className="flex items-center gap-2">
                  <Check className="h-4 w-4 text-emerald-600" />
                  <span>{t("settings.backup.export.includesProfile")}</span>
                </div>
              </div>
            </div>

            <div className="rounded-md border p-3">
              <div className="flex items-start justify-between gap-4">
                <div className="space-y-1">
                  <div className="font-medium text-foreground">
                    {includeRecordings
                      ? t("settings.backup.export.scopeCompleteLabel")
                      : t("settings.backup.export.scopeSmallerLabel")}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {includeRecordings
                      ? t("settings.backup.export.scopeCompleteDescription")
                      : t("settings.backup.export.scopeSmallerDescription")}
                  </div>
                </div>
                <Switch
                  checked={includeRecordings}
                  onCheckedChange={setIncludeRecordings}
                  disabled={isWorking || isEstimating}
                />
              </div>
            </div>

            <div className="rounded-md border p-3 text-xs text-muted-foreground">
              {isEstimating ? (
                <div className="flex items-center gap-2">
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  <span>{t("settings.backup.export.estimateLoading")}</span>
                </div>
              ) : backupEstimate ? (
                <div className="space-y-1">
                  <div>
                    {t("settings.backup.export.estimatedSize")}:{" "}
                    <span className="font-medium text-foreground">
                      {formatBytes(selectedEstimatedBytes ?? 0)}
                    </span>
                  </div>
                  <div>
                    {t("settings.backup.export.recordingSavings", {
                      size: formatBytes(backupEstimate.difference_bytes),
                    })}
                  </div>
                </div>
              ) : (
                <div>{t("settings.backup.export.estimateUnavailable")}</div>
              )}
            </div>

            <div className="rounded-md border p-3 text-xs text-muted-foreground">
              {t("settings.backup.maintenanceNotice")}
            </div>
          </div>

          <DialogFooter className="gap-2 sm:space-x-0">
            <Button
              type="button"
              variant="outline"
              onClick={() => setShowCreateDialog(false)}
              disabled={isWorking}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="button"
              onClick={handleCreateBackupConfirm}
              disabled={isWorking || isEstimating}
            >
              {isCreateLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t("common.loading")}
                </>
              ) : (
                t("settings.backup.export.createAction")
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={showPreflightDialog}
        onOpenChange={(open) => {
          if (!isWorking) {
            if (!open) {
              setIsBackupDetailsOpen(false);
            }
            setShowPreflightDialog(open);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("settings.backup.restore.preflightTitle")}</DialogTitle>
            <DialogDescription>
              {preflightReport?.can_apply
                ? t("settings.backup.restore.canContinue")
                : t("settings.backup.restore.cannotContinue")}
            </DialogDescription>
          </DialogHeader>

          {preflightReport?.summary ? (
            <div className="space-y-3">
              <div className="rounded-md border bg-muted/20 p-3 space-y-2">
                <div className="text-sm font-medium text-foreground">
                  {t("settings.backup.restore.impact.title")}
                </div>
                <div className="text-xs leading-relaxed text-muted-foreground">
                  {restoreImpactState.hasLocalSnapshot && restoreImpactState.isFreshInstall
                    ? t("settings.backup.restore.impact.freshInstall")
                    : t("settings.backup.restore.impact.replaceCurrent")}
                </div>
                {restoreImpactState.willRemoveRecordings ? (
                  <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">
                    {t("settings.backup.restore.impact.recordingsWillBeRemoved", {
                      count: restoreImpactState.localRecordingFiles,
                    })}
                  </div>
                ) : null}
              </div>

              <div className="rounded-md border p-3 space-y-3">
                <div className="text-sm text-foreground">
                  {t("settings.backup.restore.summary.createdAtFriendly", {
                    createdAt:
                      formattedPreflightCreatedAt || preflightReport.summary.created_at,
                  })}
                </div>
                <div className="grid gap-2 sm:grid-cols-3">
                  <div className="rounded-md border bg-muted/20 p-2.5">
                    <div className="text-[11px] uppercase tracking-wide text-muted-foreground">
                      {t("settings.backup.restore.summary.historyEntries")}
                    </div>
                    <div className="mt-1 text-lg font-semibold text-foreground">
                      {preflightReport.summary.counts.history_entries.toLocaleString(
                        i18n.language,
                      )}
                    </div>
                  </div>
                  <div className="rounded-md border bg-muted/20 p-2.5">
                    <div className="text-[11px] uppercase tracking-wide text-muted-foreground">
                      {t("settings.backup.restore.summary.dictionaryEntries")}
                    </div>
                    <div className="mt-1 text-lg font-semibold text-foreground">
                      {preflightReport.summary.counts.dictionary_entries.toLocaleString(
                        i18n.language,
                      )}
                    </div>
                  </div>
                  <div className="rounded-md border bg-muted/20 p-2.5">
                    <div className="text-[11px] uppercase tracking-wide text-muted-foreground">
                      {t("settings.backup.restore.summary.recordingFiles")}
                    </div>
                    <div className="mt-1 text-lg font-semibold text-foreground">
                      {preflightReport.summary.counts.recording_files.toLocaleString(
                        i18n.language,
                      )}
                    </div>
                  </div>
                </div>
                {!preflightReport.summary.includes_recordings &&
                !restoreImpactState.willRemoveRecordings ? (
                  <div className="rounded-md border border-dashed bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
                    {t("settings.backup.restore.summary.excludesRecordings")}
                  </div>
                ) : null}

                <Collapsible
                  open={isBackupDetailsOpen}
                  onOpenChange={setIsBackupDetailsOpen}
                  className="overflow-hidden rounded-md border border-border/70 bg-muted/10"
                >
                  <CollapsibleTrigger asChild>
                    <button
                      type="button"
                      className="group flex w-full items-center justify-between gap-3 px-3 py-2.5 text-left transition-colors hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
                    >
                      <div className="text-sm font-medium text-foreground">
                        {isBackupDetailsOpen
                          ? t("settings.backup.restore.summary.detailsToggleHide")
                          : t("settings.backup.restore.summary.detailsToggle")}
                      </div>
                      <ChevronDown
                        className={`h-4 w-4 text-muted-foreground transition-transform duration-200 ${
                          isBackupDetailsOpen ? "rotate-180" : ""
                        }`}
                      />
                    </button>
                  </CollapsibleTrigger>
                  <CollapsibleContent className="border-t border-border/60 pt-1">
                    <div className="rounded-md bg-background/20 px-4 py-3">
                      <ul className="list-disc space-y-2.5 pl-3 text-sm text-muted-foreground marker:text-muted-foreground/80">
                        <li>
                          {t("settings.backup.restore.summary.appVersion")}:{" "}
                          {preflightReport.summary.created_with_app_version}
                        </li>
                        <li>
                          {t("settings.backup.restore.summary.platform")}:{" "}
                          {preflightReport.summary.platform}
                        </li>
                      </ul>
                    </div>
                  </CollapsibleContent>
                </Collapsible>
              </div>
            </div>
          ) : null}

          {preflightCompatibilityNote ? (
            <div className="rounded-md border bg-muted/20 p-3 space-y-1">
              <div className="text-xs uppercase tracking-wide text-muted-foreground">
                {t("settings.backup.restore.compatibilityLabel")}
              </div>
              <div className="text-sm text-muted-foreground leading-relaxed">
                {preflightCompatibilityNote}
              </div>
            </div>
          ) : null}

          {preflightReport?.blocking_findings?.length ? (
            <div className="space-y-1">
              <div className="text-sm font-medium text-destructive">
                {t("settings.backup.restore.blockingTitle")}
              </div>
              {preflightReport.blocking_findings.map((finding, index) => (
                <div key={`${finding.code}-${index}`} className="text-sm text-destructive">
                  - {formatFindingMessage(finding, "blocking")}
                </div>
              ))}
            </div>
          ) : null}

          {visibleRecoverableFindings.length ? (
            <div className="space-y-1">
              <div className="text-sm font-medium text-amber-700">
                {t("settings.backup.restore.warningsTitle")}
              </div>
              {visibleRecoverableFindings.map((finding, index) => (
                <div key={`${finding.code}-${index}`} className="text-sm text-amber-800">
                  - {formatFindingMessage(finding, "recoverable")}
                </div>
              ))}
            </div>
          ) : null}

          <DialogFooter className="gap-2 sm:space-x-0">
            <Button
              type="button"
              variant="outline"
              onClick={() => setShowPreflightDialog(false)}
              disabled={isWorking}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={runRestore}
              disabled={isWorking || !preflightReport?.can_apply}
            >
              {isRestoreApplyLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t("common.loading")}
                </>
              ) : (
                t("settings.backup.restore.confirmAction")
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
