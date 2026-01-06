import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, Trash2, RotateCcw, X, Loader2, Check } from "lucide-react";

import { useMlxModels } from "@/hooks/useMlxModels";
import type { MlxModelInfo } from "@/bindings";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

/** Format bytes to human readable size */
function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
  }
  return `${(bytes / 1024).toFixed(0)} KB`;
}

interface ModelItemProps {
  model: MlxModelInfo;
  isSelected: boolean;
  onSelect: () => void;
  onDownload: () => void;
  onDelete: () => void;
  onCancel: () => void;
  onRetry: () => void;
}

const ModelItem: React.FC<ModelItemProps> = ({
  model,
  isSelected,
  onSelect,
  onDownload,
  onDelete,
  onCancel,
  onRetry,
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
          </div>
          <p className="text-xs text-mid-gray mt-0.5 line-clamp-2">
            {model.description}
          </p>
          <div className="flex items-center gap-3 mt-1.5 text-xs text-mid-gray">
            <span>
              {t("settings.postProcessing.mlx.parameters")}: {model.parameters}
            </span>
            <span>
              {t("settings.postProcessing.mlx.size")}: {formatSize(model.size_bytes)}
            </span>
            <span className={statusColor}>{statusLabel}</span>
          </div>

          {/* Download progress bar */}
          {isDownloading && (
            <div className="mt-2">
              <div className="h-1.5 bg-mid-gray/20 rounded-full overflow-hidden">
                <div
                  className="h-full bg-accent transition-all duration-300"
                  style={{ width: `${model.download_progress * 100}%` }}
                />
              </div>
              <span className="text-xs text-mid-gray mt-0.5">
                {Math.round(model.download_progress * 100)}%
              </span>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1">
          {canSelect && (
            <Button
              variant={isSelected ? "primary" : "secondary"}
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
            <Button variant="primary" size="sm" onClick={onDownload}>
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
                variant="primary"
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

        {models.map((model) => (
          <ModelItem
            key={model.id}
            model={model}
            isSelected={selectedModelId === model.id}
            onSelect={() => onModelSelect(model.id)}
            onDownload={() => downloadModel(model.id)}
            onDelete={() => deleteModel(model.id)}
            onCancel={cancelDownload}
            onRetry={retryDownload}
          />
        ))}
      </div>
    </SettingContainer>
  );
};

MlxModelSelector.displayName = "MlxModelSelector";
