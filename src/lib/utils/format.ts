export const formatModelSize = (sizeMb: number | null | undefined): string => {
  if (!sizeMb || !Number.isFinite(sizeMb) || sizeMb <= 0) {
    return "Unknown size";
  }

  if (sizeMb >= 1024) {
    const sizeGb = sizeMb / 1024;
    const formatter = new Intl.NumberFormat(undefined, {
      minimumFractionDigits: sizeGb >= 10 ? 0 : 1,
      maximumFractionDigits: sizeGb >= 10 ? 0 : 1,
    });
    return `${formatter.format(sizeGb)} GB`;
  }

  const formatter = new Intl.NumberFormat(undefined, {
    minimumFractionDigits: sizeMb >= 100 ? 0 : 1,
    maximumFractionDigits: sizeMb >= 100 ? 0 : 1,
  });

  return `${formatter.format(sizeMb)} MB`;
};

/** Format bytes to human readable size (e.g., "2.2 GB", "400.0 MB") */
export function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(0)} KB`;
  }
  return `${bytes} B`;
}

/** Format download speed to human readable format (in bits per second, e.g., "50.5 Mbps") */
export function formatSpeed(bytesPerSec: number): string {
  const bitsPerSec = bytesPerSec * 8;
  if (bitsPerSec >= 1000000) {
    return `${(bitsPerSec / 1000000).toFixed(1)} Mbps`;
  }
  if (bitsPerSec >= 1000) {
    return `${(bitsPerSec / 1000).toFixed(0)} Kbps`;
  }
  return `${bitsPerSec.toFixed(0)} bps`;
}

/** Format remaining time estimate based on bytes remaining and speed */
export function formatEta(remainingBytes: number, speedBytesPerSec: number): string {
  if (speedBytesPerSec <= 0) return "calculating...";

  const seconds = Math.ceil(remainingBytes / speedBytesPerSec);
  if (seconds < 60) return `${seconds}s left`;
  if (seconds < 3600) return `${Math.ceil(seconds / 60)}m left`;
  const hours = Math.floor(seconds / 3600);
  const mins = Math.ceil((seconds % 3600) / 60);
  return `${hours}h ${mins}m left`;
}
