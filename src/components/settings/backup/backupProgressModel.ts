const BACKUP_ETA_HISTORY_STORAGE_KEY = "settings.backup.etaHistory.v1";
const BACKUP_ETA_HISTORY_MAX_SAMPLES = 40;
const BACKUP_ETA_HISTORY_RETENTION_MS = 120 * 24 * 60 * 60 * 1000;
const BACKUP_ETA_MIN_SECONDS = 60;
const BACKUP_ETA_MAX_SECONDS = 7200;

type BackupScope = "complete" | "smaller";

export interface BackupEtaFeatures {
  scope: BackupScope;
  estimated_size_bytes: number;
  recording_files: number;
  history_entries: number;
}

export interface BackupEtaHistorySample extends BackupEtaFeatures {
  duration_seconds: number;
  completed_at_ms: number;
}

function getLocalStorage(): Storage | null {
  try {
    if (
      typeof globalThis === "undefined" ||
      !("localStorage" in globalThis) ||
      !globalThis.localStorage
    ) {
      return null;
    }
    return globalThis.localStorage;
  } catch {
    return null;
  }
}

function clampDurationSeconds(value: number): number {
  if (!Number.isFinite(value)) {
    return BACKUP_ETA_MIN_SECONDS;
  }
  return Math.max(BACKUP_ETA_MIN_SECONDS, Math.min(BACKUP_ETA_MAX_SECONDS, value));
}

function isValidScope(value: unknown): value is BackupScope {
  return value === "complete" || value === "smaller";
}

function sanitizeSample(
  sample: Partial<BackupEtaHistorySample> | null | undefined,
): BackupEtaHistorySample | null {
  if (!sample || !isValidScope(sample.scope)) {
    return null;
  }

  const estimated_size_bytes = Number(sample.estimated_size_bytes);
  const recording_files = Number(sample.recording_files);
  const history_entries = Number(sample.history_entries);
  const duration_seconds = Number(sample.duration_seconds);
  const completed_at_ms = Number(sample.completed_at_ms);

  if (
    !Number.isFinite(estimated_size_bytes) ||
    !Number.isFinite(recording_files) ||
    !Number.isFinite(history_entries) ||
    !Number.isFinite(duration_seconds) ||
    !Number.isFinite(completed_at_ms)
  ) {
    return null;
  }

  if (
    estimated_size_bytes < 0 ||
    recording_files < 0 ||
    history_entries < 0 ||
    duration_seconds <= 0 ||
    completed_at_ms <= 0
  ) {
    return null;
  }

  return {
    scope: sample.scope,
    estimated_size_bytes,
    recording_files,
    history_entries,
    duration_seconds,
    completed_at_ms,
  };
}

function applyHistoryRetention(samples: BackupEtaHistorySample[]): BackupEtaHistorySample[] {
  const cutoff = Date.now() - BACKUP_ETA_HISTORY_RETENTION_MS;
  return samples
    .filter((sample) => sample.completed_at_ms >= cutoff)
    .sort((a, b) => b.completed_at_ms - a.completed_at_ms)
    .slice(0, BACKUP_ETA_HISTORY_MAX_SAMPLES);
}

function quantile(values: number[], q: number): number | null {
  if (values.length === 0) {
    return null;
  }

  const sorted = [...values].sort((a, b) => a - b);
  if (sorted.length === 1) {
    return sorted[0];
  }

  const safeQ = Math.max(0, Math.min(1, q));
  const position = (sorted.length - 1) * safeQ;
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  if (lower === upper) {
    return sorted[lower];
  }
  const fraction = position - lower;
  return sorted[lower] + (sorted[upper] - sorted[lower]) * fraction;
}

function fallbackDurationSeconds(features: BackupEtaFeatures): number {
  const estimatedSizeMiB = features.estimated_size_bytes / (1024 * 1024);
  const base = features.scope === "complete" ? 45 : 25;
  const size_term =
    estimatedSizeMiB / (features.scope === "complete" ? 4.0 : 7.0);
  const recordings_term = features.recording_files * 0.08;
  const history_term = Math.min(120, (features.history_entries / 1000) * 0.25);
  const predicted = (base + size_term + recordings_term + history_term) * 1.15;
  return clampDurationSeconds(predicted);
}

function log2Distance(a: number, b: number): number {
  return Math.abs(Math.log2((a + 1) / (b + 1)));
}

function adaptiveDurationSeconds(
  features: BackupEtaFeatures,
  historySamples: BackupEtaHistorySample[],
): number | null {
  const sameScope = historySamples.filter(
    (sample) => sample.scope === features.scope,
  );
  if (sameScope.length < 3) {
    return null;
  }

  const nearest = sameScope
    .map((sample) => {
      const distance =
        log2Distance(features.estimated_size_bytes, sample.estimated_size_bytes) +
        0.35 *
          log2Distance(features.recording_files, sample.recording_files);
      return { sample, distance };
    })
    .sort((a, b) => a.distance - b.distance)
    .slice(0, 7);

  if (nearest.length < 3) {
    return null;
  }

  const durationP65 = quantile(
    nearest.map(({ sample }) => sample.duration_seconds),
    0.65,
  );
  if (durationP65 == null) {
    return null;
  }

  return clampDurationSeconds(durationP65);
}

export function predictTotalDurationSeconds(
  features: BackupEtaFeatures,
  historySamples: BackupEtaHistorySample[] = [],
): number {
  const fallback = fallbackDurationSeconds(features);
  const adaptive = adaptiveDurationSeconds(features, historySamples);
  if (adaptive == null) {
    return fallback;
  }
  return clampDurationSeconds(Math.max(fallback, adaptive));
}

export function loadBackupEtaHistory(): BackupEtaHistorySample[] {
  const storage = getLocalStorage();
  if (!storage) {
    return [];
  }

  const raw = storage.getItem(BACKUP_ETA_HISTORY_STORAGE_KEY);
  if (!raw) {
    return [];
  }

  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }
    const sanitized = parsed
      .map((sample) => sanitizeSample(sample))
      .filter((sample): sample is BackupEtaHistorySample => sample != null);
    return applyHistoryRetention(sanitized);
  } catch {
    return [];
  }
}

export function persistBackupEtaHistorySample(sample: BackupEtaHistorySample): void {
  const storage = getLocalStorage();
  if (!storage) {
    return;
  }

  const sanitizedSample = sanitizeSample(sample);
  if (!sanitizedSample) {
    return;
  }

  const next = applyHistoryRetention([
    ...loadBackupEtaHistory(),
    sanitizedSample,
  ]);
  storage.setItem(BACKUP_ETA_HISTORY_STORAGE_KEY, JSON.stringify(next));
}

export function etaBucketKeyForRemainingSeconds(remainingSeconds: number): string {
  if (!Number.isFinite(remainingSeconds) || remainingSeconds <= 0) {
    return "settings.backup.operation.etaBuckets.overrun";
  }
  if (remainingSeconds <= 120) {
    return "settings.backup.operation.etaBuckets.underTwoMinutes";
  }
  if (remainingSeconds <= 300) {
    return "settings.backup.operation.etaBuckets.twoToFiveMinutes";
  }
  if (remainingSeconds <= 600) {
    return "settings.backup.operation.etaBuckets.fiveToTenMinutes";
  }
  if (remainingSeconds <= 900) {
    return "settings.backup.operation.etaBuckets.tenToFifteenMinutes";
  }
  return "settings.backup.operation.etaBuckets.overFifteenMinutes";
}

export function isBackupRestoreCancelledError(message: string): boolean {
  return /cancelled safely/i.test(message);
}

export { BACKUP_ETA_HISTORY_STORAGE_KEY };
