import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, Trash2, RotateCcw, X, Loader2, Check, FolderOpen } from "lucide-react";

import { useMlxModels } from "@/hooks/useMlxModels";
import { commands, type MlxModelInfo } from "@/bindings";
import { Button } from "@/components/shared/ui/button";
import { SettingContainer } from "../ui/SettingContainer";

/** Format bytes to human readable size */
function formatSize(bytes: number): string {
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

/** Format speed to human readable format (in bits per second) */
function formatSpeed(bytesPerSec: number): string {
  const bitsPerSec = bytesPerSec * 8;
  if (bitsPerSec >= 1000000) {
    return `${(bitsPerSec / 1000000).toFixed(1)} Mbps`;
  }
  if (bitsPerSec >= 1000) {
    return `${(bitsPerSec / 1000).toFixed(0)} Kbps`;
  }
  return `${bitsPerSec.toFixed(0)} bps`;
}

/** Format remaining time */
function formatEta(remainingBytes: number, speedBytesPerSec: number): string {
  if (speedBytesPerSec <= 0) return "calculating...";

  const seconds = Math.ceil(remainingBytes / speedBytesPerSec);
  if (seconds < 60) return `${seconds}s left`;
  if (seconds < 3600) return `${Math.ceil(seconds / 60)}m left`;
  const hours = Math.floor(seconds / 3600);
  const mins = Math.ceil((seconds % 3600) / 60);
  return `${hours}h ${mins}m left`;
}

interface DownloadProgressInfo {
  downloadedBytes: number;
  totalBytes: number;
  speedBytesPerSec: number;
  currentFile: string;
}

interface ModelItemProps {
  model: MlxModelInfo;
  isSelected: boolean;
  downloadProgress: DownloadProgressInfo | null;
  onSelect: () => void;
  onDownload: () => void;
  onDelete: () => void;
  onCancel: () => void;
  onRetry: () => void;
  onShowInFinder: () => void;
}

const ModelItem: React.FC<ModelItemProps> = ({
  model,
  isSelected,
  downloadProgress,
  onSelect,
  onDownload,
  onDelete,
  onCancel,
  onRetry,
  onShowInFinder,
}) => {
  const { t } = useTranslation();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const statusLabel = {
    not_downloaded: t("settings.postProcessing.mlx.notDownloaded"),
    downloading: t("settings.postProcessing.mlx.downloading"),
    download_failed: t("settings.postProcessing.mlx.failed"),
    downloaded: t("settings.postProcessing.mlx.downloaded"),
    loading: t("settings.postProcessing.mlx.loading"),
    ready: t("settings.postProcessing.mlx.ready"),
    load_failed: t("settings.postProcessing.mlx.failed"),
  }[model.status];

  const statusColor = {
    not_downloaded: "text-mid-gray",
    downloading: "text-blue-500",
    download_failed: "text-red-500",
    downloaded: "text-green-500",
    loading: "text-blue-500",
    ready: "text-green-500",
    load_failed: "text-red-500",
  }[model.status];

  const canDownload = model.status === "not_downloaded";
  const canDelete =
    model.status === "downloaded" || model.status === "download_failed";
  const isDownloading = model.status === "downloading";
  const canRetry = model.status === "download_failed";
  const canSelect = model.status === "downloaded" || model.status === "ready";

  return (
    <div
      className={`p-3 rounded-lg border transition-colors ${
        isSelected
          ? "border-accent bg-accent/5"
          : "border-mid-gray/20 hover:border-mid-gray/40"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h4 className="font-medium text-sm truncate">
              {model.display_name}
            </h4>
            {model.is_default && (
              <span className="text-xs px-1.5 py-0.5 bg-accent/20 text-accent rounded">
                {t("settings.postProcessing.mlx.recommended")}
              </span>
            )}
            {isSelected && (
              <span className="text-xs px-1.5 py-0.5 bg-green-500/20 text-green-500 rounded">
                {t("settings.postProcessing.mlx.default")}
              </span>
            )}
          </div>
          <p className="text-xs text-mid-gray mt-0.5 line-clamp-2">
            {model.description}
          </p>
          <div className="flex items-center gap-3 mt-1.5 text-xs text-mid-gray">
            <span>
              {t("settings.postProcessing.mlx.ram")}: {model.parameters}
            </span>
            <span>
              {t("settings.postProcessing.mlx.size")}: {formatSize(model.size_bytes)}
            </span>
            <span className={statusColor}>{statusLabel}</span>
            {(model.status === "downloaded" || model.status === "ready") && (
              <button
                onClick={onShowInFinder}
                className="inline-flex items-center gap-1 text-mid-gray hover:text-foreground transition-colors cursor-pointer"
              >
                <FolderOpen className="h-3 w-3" />
                <span>{t("settings.postProcessing.mlx.showInFinder")}</span>
              </button>
            )}
          </div>

          {/* Download progress bar with detailed info */}
          {isDownloading && (
            <div className="mt-2 space-y-1">
              <div className="h-3 bg-mid-gray/30 rounded-full overflow-hidden">
                <div
                  className="h-full bg-blue-500 transition-all duration-150 ease-out"
                  style={{
                    width: downloadProgress && downloadProgress.totalBytes > 0
                      ? `${Math.max(2, (downloadProgress.downloadedBytes / downloadProgress.totalBytes) * 100)}%`
                      : `${Math.max(2, model.download_progress * 100)}%`
                  }}
                />
              </div>
              <div className="flex items-center justify-between text-xs text-mid-gray">
                <span>
                  {downloadProgress
                    ? `${formatSize(downloadProgress.downloadedBytes)} / ${formatSize(downloadProgress.totalBytes)}`
                    : `${Math.round(model.download_progress * 100)}%`}
                </span>
                {downloadProgress && downloadProgress.speedBytesPerSec > 0 && (
                  <span className="flex items-center gap-2">
                    <span className="text-blue-400">
                      {formatSpeed(downloadProgress.speedBytesPerSec)}
                    </span>
                    <span>
                      {formatEta(
                        downloadProgress.totalBytes -
                          downloadProgress.downloadedBytes,
                        downloadProgress.speedBytesPerSec
                      )}
                    </span>
                  </span>
                )}
              </div>
              {downloadProgress?.currentFile && (
                <p className="text-xs text-mid-gray/70 truncate">
                  {downloadProgress.currentFile}
                </p>
              )}
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1">
          {canSelect && (
            <Button
              variant={isSelected ? "default" : "secondary"}
              size="sm"
              onClick={onSelect}
              disabled={isSelected}
            >
              {isSelected ? (
                <Check className="h-3.5 w-3.5" />
              ) : (
                t("settings.postProcessing.mlx.select")
              )}
            </Button>
          )}

          {canDownload && (
            <Button variant="default" size="sm" onClick={onDownload}>
              <Download className="h-3.5 w-3.5 mr-1" />
              {t("settings.postProcessing.mlx.download")}
            </Button>
          )}

          {isDownloading && (
            <Button variant="secondary" size="sm" onClick={onCancel}>
              <X className="h-3.5 w-3.5 mr-1" />
              {t("settings.postProcessing.mlx.cancel")}
            </Button>
          )}

          {canRetry && (
            <Button variant="secondary" size="sm" onClick={onRetry}>
              <RotateCcw className="h-3.5 w-3.5 mr-1" />
              {t("settings.postProcessing.mlx.retry")}
            </Button>
          )}

          {canDelete && !showDeleteConfirm && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setShowDeleteConfirm(true)}
            >
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          )}

          {showDeleteConfirm && (
            <div className="flex items-center gap-1">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setShowDeleteConfirm(false)}
              >
                {t("settings.postProcessing.mlx.cancelDelete")}
              </Button>
              <Button
                variant="default"
                size="sm"
                onClick={() => {
                  onDelete();
                  setShowDeleteConfirm(false);
                }}
                className="bg-red-500 hover:bg-red-600"
              >
                {t("settings.postProcessing.mlx.confirmDelete")}
              </Button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

interface MlxModelSelectorProps {
  selectedModelId: string | null;
  onModelSelect: (modelId: string) => void;
}

export const MlxModelSelector: React.FC<MlxModelSelectorProps> = ({
  selectedModelId,
  onModelSelect,
}) => {
  const { t } = useTranslation();
  const {
    models,
    isLoading,
    error,
    lastError,
    downloadProgress,
    downloadingModelId,
    downloadModel,
    cancelDownload,
    retryDownload,
    deleteModel,
  } = useMlxModels();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-6">
        <Loader2 className="h-5 w-5 animate-spin text-mid-gray" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4 bg-red-500/10 border border-red-500/50 rounded-lg">
        <p className="text-sm text-red-500">{error}</p>
      </div>
    );
  }

  return (
    <SettingContainer
      title={t("settings.postProcessing.mlx.title")}
      description={t("settings.postProcessing.mlx.description")}
      descriptionMode="inline"
      layout="stacked"
      grouped={true}
    >
      <div className="space-y-2">
        {lastError && (
          <div className="p-2 bg-red-500/10 border border-red-500/50 rounded text-xs text-red-500">
            {lastError}
          </div>
        )}

        {/* Sort models: selected first, then by default flag */}
        {[...models]
          .sort((a, b) => {
            // Selected model first
            if (selectedModelId === a.id) return -1;
            if (selectedModelId === b.id) return 1;
            // Then by default flag
            if (a.is_default && !b.is_default) return -1;
            if (!a.is_default && b.is_default) return 1;
            return 0;
          })
          .map((model) => (
          <ModelItem
            key={model.id}
            model={model}
            isSelected={selectedModelId === model.id}
            downloadProgress={
              downloadingModelId === model.id ? downloadProgress : null
            }
            onSelect={() => onModelSelect(model.id)}
            onDownload={() => downloadModel(model.id)}
            onDelete={() => deleteModel(model.id)}
            onCancel={cancelDownload}
            onRetry={retryDownload}
            onShowInFinder={() => commands.mlxOpenModelsDir(model.id)}
          />
        ))}
      </div>
    </SettingContainer>
  );
};

MlxModelSelector.displayName = "MlxModelSelector";

