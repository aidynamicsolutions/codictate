import { useCallback, useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { commands, type MlxModelInfo, type MlxModelStatus } from "@/bindings";

/**
 * MLX model state event payload from the backend
 */
type MlxModelStateEvent =
  | { event_type: "download_started"; model_id: string; total_bytes: number }
  | {
      event_type: "download_progress";
      model_id: string;
      progress: number;
      downloaded_bytes: number;
      total_bytes: number;
      speed_bytes_per_sec: number;
      current_file: string;
    }
  | { event_type: "download_completed"; model_id: string }
  | { event_type: "download_failed"; model_id: string; error: string }
  | { event_type: "download_cancelled"; model_id: string }
  | { event_type: "loading_started"; model_id: string }
  | { event_type: "loading_completed"; model_id: string }
  | { event_type: "loading_failed"; model_id: string; error: string }
  | { event_type: "unloaded"; model_id: string }
  | { event_type: "error"; model_id: string; error: string };

/** Download progress details for UI display */
interface DownloadProgress {
  /** Downloaded bytes so far */
  downloadedBytes: number;
  /** Total bytes to download */
  totalBytes: number;
  /** Download speed in bytes per second */
  speedBytesPerSec: number;
  /** Current file being downloaded */
  currentFile: string;
}

interface UseMlxModelsReturn {
  /** List of all available MLX models */
  models: MlxModelInfo[];
  /** Loading state for initial models fetch */
  isLoading: boolean;
  /** Error message if models fetch failed */
  error: string | null;
  /** Currently downloading model ID (if any) */
  downloadingModelId: string | null;
  /** Detailed download progress info */
  downloadProgress: DownloadProgress | null;
  /** Last error message from download/load operation */
  lastError: string | null;
  /** Start downloading a model */
  downloadModel: (modelId: string) => Promise<void>;
  /** Cancel the current download */
  cancelDownload: () => Promise<void>;
  /** Retry a failed download */
  retryDownload: () => Promise<void>;
  /** Delete a downloaded model */
  deleteModel: (modelId: string) => Promise<void>;
  /** Refresh the models list */
  refreshModels: () => Promise<void>;
  /** Select a model (switches to it) */
  selectModel: (modelId: string) => Promise<void>;
}

/**
 * Hook for managing MLX local AI models.
 * Provides model listing, download, cancel, retry, and delete functionality.
 */
export function useMlxModels(): UseMlxModelsReturn {
  const [models, setModels] = useState<MlxModelInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [downloadingModelId, setDownloadingModelId] = useState<string | null>(
    null
  );
  const [downloadProgress, setDownloadProgress] =
    useState<DownloadProgress | null>(null);
  const [lastError, setLastError] = useState<string | null>(null);

  const refreshModels = useCallback(async () => {
    try {
      const result = await commands.mlxListModels();
      if (result.status === "ok") {
        setModels(result.data);
        setError(null);
      } else {
        setError(result.error);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const downloadModel = useCallback(async (modelId: string) => {
    setDownloadingModelId(modelId);
    setLastError(null);
    setDownloadProgress(null);
    const result = await commands.mlxDownloadModel(modelId);
    if (result.status === "error") {
      // Don't show "cancelled" as an error - it's intentional user action
      if (!result.error.toLowerCase().includes("cancelled")) {
        setLastError(result.error);
      }
    }
    // State will be updated via events
  }, []);

  const cancelDownload = useCallback(async () => {
    const result = await commands.mlxCancelDownload();
    if (result.status === "error") {
      setLastError(result.error);
    }
  }, []);

  const retryDownload = useCallback(async () => {
    setLastError(null);
    const result = await commands.mlxRetryDownload();
    if (result.status === "error") {
      setLastError(result.error);
    }
  }, []);

  const deleteModel = useCallback(async (modelId: string) => {
    const result = await commands.mlxDeleteModel(modelId);
    if (result.status === "error") {
      setLastError(result.error);
    } else {
      // Refresh to get updated status
      await commands.mlxListModels().then((r) => {
        if (r.status === "ok") setModels(r.data);
      });
    }
  }, []);

  const selectModel = useCallback(async (modelId: string) => {
    const result = await commands.mlxSwitchModel(modelId);
    if (result.status === "error") {
      setLastError(result.error);
    }
  }, []);

  // Update local model state based on event
  const handleModelEvent = useCallback((event: MlxModelStateEvent) => {
    setModels((prev) =>
      prev.map((model) => {
        if (model.id !== event.model_id) return model;

        let status: MlxModelStatus = model.status;
        let progress = model.download_progress;

        switch (event.event_type) {
          case "download_started":
            status = "downloading";
            progress = 0;
            setDownloadProgress({
              downloadedBytes: 0,
              totalBytes: event.total_bytes,
              speedBytesPerSec: 0,
              currentFile: "",
            });
            break;
          case "download_progress":
            status = "downloading";
            progress = event.progress;
            setDownloadProgress({
              downloadedBytes: event.downloaded_bytes,
              totalBytes: event.total_bytes,
              speedBytesPerSec: event.speed_bytes_per_sec,
              currentFile: event.current_file,
            });
            break;
          case "download_completed":
            status = "downloaded";
            progress = 1;
            setDownloadingModelId(null);
            setDownloadProgress(null);
            break;
          case "download_failed":
            status = "download_failed";
            setDownloadingModelId(null);
            setDownloadProgress(null);
            setLastError(event.error);
            break;
          case "download_cancelled":
            status = "not_downloaded";
            progress = 0;
            setDownloadingModelId(null);
            setDownloadProgress(null);
            break;
          case "loading_started":
            status = "loading";
            break;
          case "loading_completed":
            status = "ready";
            break;
          case "loading_failed":
            status = "load_failed";
            setLastError(event.error);
            break;
          case "unloaded":
            status = "downloaded";
            break;
          case "error":
            setLastError(event.error);
            break;
        }

        return { ...model, status, download_progress: progress };
      })
    );
  }, []);

  // Initial models fetch
  useEffect(() => {
    refreshModels();
  }, [refreshModels]);

  // Listen for model state events
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<MlxModelStateEvent>("mlx-model-state-changed", (event) => {
      handleModelEvent(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [handleModelEvent]);

  return {
    models,
    isLoading,
    error,
    downloadingModelId,
    downloadProgress,
    lastError,
    downloadModel,
    cancelDownload,
    retryDownload,
    deleteModel,
    refreshModels,
    selectModel,
  };
}

