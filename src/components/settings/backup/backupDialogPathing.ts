const BACKUP_LAST_DIRECTORY_STORAGE_KEY = "settings.backup.lastDirectory";
const BACKUP_FILE_EXTENSION = "codictatebackup";

export type BackupFilenameScope = "complete" | "smaller";

const WINDOWS_DRIVE_ROOT_PATTERN = /^[A-Za-z]:\/$/;
const WINDOWS_DRIVE_PATTERN = /^[A-Za-z]:$/;

function normalizePathSeparators(value: string): string {
  return value.replace(/\\/g, "/");
}

function trimTrailingSeparators(value: string): string {
  let trimmed = value;
  while (trimmed.length > 1 && trimmed.endsWith("/")) {
    if (WINDOWS_DRIVE_ROOT_PATTERN.test(trimmed)) {
      break;
    }
    trimmed = trimmed.slice(0, -1);
  }
  return trimmed;
}

function normalizeDirectoryPath(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  const normalized = trimTrailingSeparators(normalizePathSeparators(value.trim()));
  if (!normalized) {
    return null;
  }

  if (normalized === "/" || WINDOWS_DRIVE_ROOT_PATTERN.test(normalized)) {
    return normalized;
  }

  if (WINDOWS_DRIVE_PATTERN.test(normalized)) {
    return `${normalized}/`;
  }

  if (!normalized.includes("/")) {
    return null;
  }

  return normalized;
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

export function extractParentDirectoryFromPath(
  filePath: string | null | undefined,
): string | null {
  if (!filePath) {
    return null;
  }

  const normalizedPath = trimTrailingSeparators(
    normalizePathSeparators(filePath.trim()),
  );
  if (!normalizedPath) {
    return null;
  }

  if (normalizedPath === "/" || WINDOWS_DRIVE_ROOT_PATTERN.test(normalizedPath)) {
    return normalizedPath;
  }

  const lastSeparatorIndex = normalizedPath.lastIndexOf("/");
  if (lastSeparatorIndex < 0) {
    return null;
  }

  if (lastSeparatorIndex === 0) {
    return "/";
  }

  const parentDirectory = normalizedPath.slice(0, lastSeparatorIndex);
  if (WINDOWS_DRIVE_PATTERN.test(parentDirectory)) {
    return `${parentDirectory}/`;
  }

  return parentDirectory;
}

export function getRememberedBackupDirectory(): string | null {
  const storage = getLocalStorage();
  if (!storage) {
    return null;
  }

  const storedValue = storage.getItem(BACKUP_LAST_DIRECTORY_STORAGE_KEY);
  return normalizeDirectoryPath(storedValue);
}

export function rememberBackupDirectoryFromFilePath(
  filePath: string | null | undefined,
): void {
  const storage = getLocalStorage();
  if (!storage) {
    return;
  }

  const directory = extractParentDirectoryFromPath(filePath);
  if (!directory) {
    return;
  }

  storage.setItem(BACKUP_LAST_DIRECTORY_STORAGE_KEY, directory);
}

export function buildBackupSaveDefaultPath(
  fileName: string,
  rememberedDirectory = getRememberedBackupDirectory(),
): string {
  const sanitizedFileName = fileName.trim();
  if (!sanitizedFileName) {
    return fileName;
  }

  const normalizedDirectory = normalizeDirectoryPath(rememberedDirectory);
  if (!normalizedDirectory) {
    return sanitizedFileName;
  }

  const separator = normalizedDirectory.endsWith("/") ? "" : "/";
  return `${normalizedDirectory}${separator}${sanitizedFileName}`;
}

export function getBackupOpenDefaultPath(): string | undefined {
  const rememberedDirectory = getRememberedBackupDirectory();
  return rememberedDirectory ?? undefined;
}

function twoDigit(value: number): string {
  return value.toString().padStart(2, "0");
}

export function formatBackupTimestamp(date = new Date()): string {
  const day = twoDigit(date.getDate());
  const month = twoDigit(date.getMonth() + 1);
  const year = date.getFullYear();
  const hours = twoDigit(date.getHours());
  const minutes = twoDigit(date.getMinutes());
  return `${day}-${month}-${year}-${hours}${minutes}`;
}

export function buildSuggestedBackupFileName(
  scope: BackupFilenameScope,
  date = new Date(),
): string {
  return `codictate-${scope}-backup-${formatBackupTimestamp(date)}.${BACKUP_FILE_EXTENSION}`;
}

export { BACKUP_LAST_DIRECTORY_STORAGE_KEY };
