import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  Download,
  FolderOpen,
  Info,
  Loader2,
  RotateCcw,
  Trash2,
  X,
} from "lucide-react";
import type { MlxModelInfo } from "@/bindings";
import { cn } from "@/lib/utils";
import { formatBytes, formatSpeed, formatEta } from "@/lib/utils/format";
import { Badge } from "@/components/shared/ui/badge";
import { Button } from "@/components/shared/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";

export type LlmModelCardStatus =
  | "downloadable"
  | "downloading"
  | "download_failed"
  | "available"
  | "active";

interface DownloadProgressInfo {
  downloadedBytes: number;
  totalBytes: number;
  speedBytesPerSec: number;
  currentFile: string;
}

interface LlmModelCardProps {
  model: MlxModelInfo;
  status: LlmModelCardStatus;
  isSelected: boolean;
  downloadProgress: DownloadProgressInfo | null;
  className?: string;
  onSelect: (modelId: string) => void;
  onDownload: (modelId: string) => void;
  onDelete: (modelId: string) => void;
  onCancel: (modelId: string) => void;
  onRetry: (modelId: string) => void;
  onShowInFinder: (modelId: string) => void;
}

export const LlmModelCard: React.FC<LlmModelCardProps> = ({
  model,
  status,
  isSelected,
  downloadProgress,
  className = "",
  onSelect,
  onDownload,
  onDelete,
  onCancel,
  onRetry,
  onShowInFinder,
}) => {
  const { t } = useTranslation();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isHovered, setIsHovered] = useState(false);

  const isClickable =
    status === "available" || status === "active" || status === "downloadable";
  const isDownloading = status === "downloading";
  const canRetry = status === "download_failed";
  const canDelete = status === "available" || status === "active";

  const baseClasses =
    "flex flex-col rounded-xl px-4 py-3 gap-2 text-left transition-all duration-200";

  const getVariantClasses = () => {
    if (isSelected) {
      return "border-2 border-logo-primary/50 bg-logo-primary/10";
    }
    if (model.is_default) {
      return "border-2 border-logo-primary/25 bg-logo-primary/5";
    }
    return "border-2 border-mid-gray/20";
  };

  const getInteractiveClasses = () => {
    if (!isClickable) return "";
    return "cursor-pointer hover:border-logo-primary/50 hover:bg-logo-primary/5 hover:shadow-lg hover:scale-[1.01] active:scale-[0.99] group";
  };

  const handleClick = () => {
    if (!isClickable) return;
    if (status === "downloadable") {
      onDownload(model.id);
    } else {
      onSelect(model.id);
    }
  };

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (showDeleteConfirm) {
      onDelete(model.id);
      setShowDeleteConfirm(false);
    } else {
      setShowDeleteConfirm(true);
    }
  };

  const handleCancelDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowDeleteConfirm(false);
  };

  return (
    <div
      onClick={handleClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" && isClickable) handleClick();
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      role={isClickable ? "button" : undefined}
      tabIndex={isClickable ? 0 : undefined}
      className={cn(baseClasses, getVariantClasses(), getInteractiveClasses(), className)}
    >
      {/* Top section: name/description */}
      <div className="flex justify-between items-center w-full">
        <div className="flex flex-col items-start flex-1 min-w-0">
          <div className="flex items-center gap-3 flex-wrap">
            <h3
              className={`text-base font-semibold text-text ${isClickable ? "group-hover:text-logo-primary" : ""} transition-colors`}
            >
              {model.display_name}
            </h3>
            {model.is_default && (
              <Badge variant="default">
                {t("settings.refine.mlx.recommended")}
              </Badge>
            )}
            {isSelected && (
              <Badge variant="default">
                <Check className="w-3 h-3 mr-1" />
                {t("modelSelector.active")}
              </Badge>
            )}
          </div>
          <p className="text-text/60 text-sm leading-relaxed">
            {model.description}
          </p>
        </div>
      </div>

      <hr className="w-full border-mid-gray/20" />

      {/* Bottom row: RAM, Size, and actions */}
      <div className="flex items-center gap-3 w-full -mb-0.5 mt-0.5 h-5">
        <span className="text-xs text-text/50">
          {t("settings.refine.mlx.ram")}: {model.parameters}
        </span>
        {/* For downloadable, only show size on right with icon */}
        {status !== "downloadable" && (
          <span className="text-xs text-text/50">
            {t("settings.refine.mlx.size")}: {formatBytes(model.size_bytes)}
          </span>
        )}

        {/* Show in Finder - only on hover for downloaded models */}
        {(status === "available" || status === "active") && isHovered && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onShowInFinder(model.id);
            }}
            className="flex items-center gap-1 text-xs text-text/50 hover:text-text transition-colors"
          >
            <FolderOpen className="w-3.5 h-3.5" />
            <span>{t("settings.refine.mlx.showInFinder")}</span>
          </button>
        )}

        {/* Download size for downloadable */}
        {status === "downloadable" && (
          <span className="flex items-center gap-1.5 ml-auto text-xs text-text/50">
            <Download className="w-3.5 h-3.5" />
            <span>{formatBytes(model.size_bytes)}</span>
          </span>
        )}

        {/* Delete button */}
        {canDelete && !showDeleteConfirm && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleDelete}
            title={t("common.delete")}
            className="flex items-center gap-1.5 ml-auto text-destructive hover:text-destructive hover:bg-destructive/10"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>{t("common.delete")}</span>
          </Button>
        )}

        {/* Delete confirmation */}
        {showDeleteConfirm && (
          <div className="flex items-center gap-1 ml-auto">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCancelDelete}
              className="text-xs"
            >
              {t("settings.refine.mlx.cancelDelete")}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleDelete}
              className="text-destructive hover:text-destructive hover:bg-destructive/10 text-xs"
            >
              {t("settings.refine.mlx.confirmDelete")}
            </Button>
          </div>
        )}

        {/* Retry button for failed downloads */}
        {canRetry && (
          <Button
            variant="ghost"
            size="sm"
            onClick={(e) => {
              e.stopPropagation();
              onRetry(model.id);
            }}
            className="flex items-center gap-1.5 ml-auto text-logo-primary hover:text-logo-primary hover:bg-logo-primary/10"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>{t("settings.refine.mlx.retry")}</span>
          </Button>
        )}
      </div>

      {/* Download progress - detailed like MLX original */}
      {isDownloading && (
        <div className="w-full mt-3">
          <div className="w-full h-1.5 bg-mid-gray/20 rounded-full overflow-hidden">
            <div
              className="h-full bg-logo-primary rounded-full transition-all duration-300"
              style={{
                width: `${
                  downloadProgress && downloadProgress.totalBytes > 0
                    ? (downloadProgress.downloadedBytes / downloadProgress.totalBytes) * 100
                    : model.download_progress * 100
                }%`,
              }}
            />
          </div>
          <div className="flex items-center justify-between text-xs mt-1">
            <span className="text-text/50">
              {downloadProgress
                ? `${formatBytes(downloadProgress.downloadedBytes)} / ${formatBytes(downloadProgress.totalBytes)}`
                : `${Math.round(model.download_progress * 100)}%`}
            </span>
            <div className="flex items-center gap-2">
              {downloadProgress && downloadProgress.speedBytesPerSec > 0 && (
                <>
                  <span className="text-blue-400 tabular-nums">
                    {formatSpeed(downloadProgress.speedBytesPerSec)}
                  </span>
                  <span className="text-text/50">
                    {formatEta(
                      downloadProgress.totalBytes - downloadProgress.downloadedBytes,
                      downloadProgress.speedBytesPerSec
                    )}
                  </span>
                </>
              )}
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation();
                  onCancel(model.id);
                }}
                className="text-destructive hover:text-destructive hover:bg-destructive/10"
              >
                {t("modelSelector.cancel")}
              </Button>
            </div>
          </div>
          {downloadProgress?.currentFile && (
            <p className="text-xs text-text/40 truncate mt-0.5">
              {downloadProgress.currentFile}
            </p>
          )}
        </div>
      )}

      {/* Failed download state */}
      {status === "download_failed" && (
        <div className="w-full mt-2 p-2 bg-destructive/10 border border-destructive/30 rounded-md">
          <p className="text-xs text-destructive">
            {t("settings.refine.mlx.failed")}
          </p>
        </div>
      )}
    </div>
  );
};

export default LlmModelCard;
