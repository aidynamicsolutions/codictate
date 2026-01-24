import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronUp, X, RefreshCw, Check } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { commands } from "@/bindings";
import {
  getTranslatedModelName,
} from "@/lib/utils/modelTranslation";

interface DownloadProgress {
  model_id: string;
  downloaded: number;
  total: number;
  percentage: number;
}

interface DownloadStats {
  speed: number;
}

interface ModelDownloadProgressProps {
  className?: string;
}

/**
 * Persistent download progress indicator that appears at bottom-right
 * when a model download is in progress during onboarding.
 */
export const ModelDownloadProgress: React.FC<ModelDownloadProgressProps> = ({
  className = "",
}) => {
  const { t } = useTranslation();
  
  const [isVisible, setIsVisible] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadStats, setDownloadStats] = useState<DownloadStats | null>(null);
  const [modelName, setModelName] = useState<string>("");
  const [hasError, setHasError] = useState(false);
  const [errorModelId, setErrorModelId] = useState<string | null>(null);
  const [isRetrying, setIsRetrying] = useState(false);
  const [isExtracting, setIsExtracting] = useState(false);
  const [isComplete, setIsComplete] = useState(false);
  
  // Speed calculation refs
  const lastProgressRef = useRef<{ downloaded: number; time: number } | null>(null);
  const speedSamplesRef = useRef<number[]>([]);
  const modelNameRef = useRef<string>("");

  // Listen for download events - use empty dependency array to match ModelSelector.tsx pattern
  useEffect(() => {
    // Track if component is mounted to prevent state updates after unmount
    let isMounted = true;

    const progressUnlisten = listen<DownloadProgress>(
      "model-download-progress",
      async (event) => {
        const progress = event.payload;
        setDownloadProgress(progress);
        setIsVisible(true);
        setHasError(false);

        // Get model name if not set (using ref to check)
        if (!modelNameRef.current) {
          try {
            const result = await commands.getModelInfo(progress.model_id);
            // Check if still mounted before updating state
            if (isMounted && result.status === "ok" && result.data) {
              const name = getTranslatedModelName(result.data, t);
              modelNameRef.current = name;
              setModelName(name);
            }
          } catch (e) {
            console.error("Failed to get model info:", e);
          }
        }

        // Calculate speed
        const now = Date.now();
        if (lastProgressRef.current) {
          const timeDiff = (now - lastProgressRef.current.time) / 1000;
          const bytesDiff = progress.downloaded - lastProgressRef.current.downloaded;
          if (timeDiff > 0 && bytesDiff > 0) {
            const speed = bytesDiff / timeDiff / (1024 * 1024); // MB/s
            speedSamplesRef.current.push(speed);
            if (speedSamplesRef.current.length > 5) {
              speedSamplesRef.current.shift();
            }
            const avgSpeed =
              speedSamplesRef.current.reduce((a, b) => a + b, 0) /
              speedSamplesRef.current.length;
            setDownloadStats({ speed: avgSpeed });
          }
        }
        lastProgressRef.current = { downloaded: progress.downloaded, time: now };
      }
    );

    const completeUnlisten = listen<string>("model-download-complete", () => {
      // Download complete, extraction will start
      // Don't hide yet - wait for extraction events
      setDownloadProgress(null);
      lastProgressRef.current = null;
      speedSamplesRef.current = [];
    });

    const extractionStartedUnlisten = listen<string>(
      "model-extraction-started",
      () => {
        setIsExtracting(true);
        setIsVisible(true);
      }
    );

    const extractionCompletedUnlisten = listen<string>(
      "model-extraction-completed",
      () => {
        // Show complete state briefly before hiding
        setIsExtracting(false);
        setIsComplete(true);
        setDownloadProgress(null);
        setDownloadStats(null);
        
        // Hide after showing complete state for 5 seconds
        setTimeout(() => {
          if (isMounted) {
            setIsVisible(false);
            setIsComplete(false);
            setModelName("");
            modelNameRef.current = "";
          }
        }, 5000);
      }
    );

    const extractionFailedUnlisten = listen<{ model_id: string; error: string }>(
      "model-extraction-failed",
      (event) => {
        setHasError(true);
        setErrorModelId(event.payload.model_id);
        setIsRetrying(false);
      }
    );

    return () => {
      isMounted = false;
      progressUnlisten.then((fn) => fn());
      completeUnlisten.then((fn) => fn());
      extractionStartedUnlisten.then((fn) => fn());
      extractionCompletedUnlisten.then((fn) => fn());
      extractionFailedUnlisten.then((fn) => fn());
    };
  }, [t]);

  // Handle retry
  const handleRetry = async () => {
    if (!errorModelId) return;
    
    setIsRetrying(true);
    setHasError(false);
    
    try {
      const result = await commands.downloadModel(errorModelId);
      if (result.status !== "ok") {
        setHasError(true);
        setIsRetrying(false);
      }
    } catch (e) {
      console.error("Retry failed:", e);
      setHasError(true);
      setIsRetrying(false);
    }
  };

  // Handle dismiss
  const handleDismiss = () => {
    setIsVisible(false);
    setDownloadProgress(null);
    setHasError(false);
  };

  if (!isVisible) return null;

  const percentage = downloadProgress?.percentage ?? 0;

  return (
    <div
      className={`fixed bottom-4 right-4 z-50 ${className}`}
    >
      <div className="bg-background border border-border rounded-lg shadow-lg overflow-hidden min-w-[280px]">
        {/* Header with collapse toggle */}
        <div
          className="flex items-center justify-between px-3 py-2 bg-muted/50 cursor-pointer hover:bg-muted/70 transition-colors"
          onClick={() => setIsCollapsed(!isCollapsed)}
        >
          <div className="flex items-center gap-2">
            {/* Status indicator dot */}
            <span className="relative flex h-2 w-2">
              {isComplete ? (
                <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
              ) : (
                <>
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-primary" />
                </>
              )}
            </span>
            <span className="text-sm font-medium text-foreground">
              {hasError 
                ? t("onboarding.downloadModel.error")
                : isComplete
                  ? t("onboarding.downloadModel.complete")
                  : isExtracting
                    ? t("modelSelector.extractingGeneric")
                    : `${t("onboarding.downloadModel.downloading")} ${Math.round(percentage)}%`
              }
            </span>
          </div>
          
          <div className="flex items-center gap-1">
            {hasError && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleDismiss();
                }}
                className="p-1 rounded hover:bg-muted transition-colors"
              >
                <X className="h-4 w-4 text-muted-foreground" />
              </button>
            )}
            <button className="p-1 rounded hover:bg-muted transition-colors">
              {isCollapsed ? (
                <ChevronUp className="h-4 w-4 text-muted-foreground" />
              ) : (
                <ChevronDown className="h-4 w-4 text-muted-foreground" />
              )}
            </button>
          </div>
        </div>

        {/* Collapsed view - just progress bar (only during download, not extraction or complete) */}
        {isCollapsed && !hasError && !isExtracting && !isComplete && downloadProgress && (
          <div className="px-3 pb-2">
            <progress
              value={percentage}
              max={100}
              className="w-full h-1.5 [&::-webkit-progress-bar]:rounded-full [&::-webkit-progress-bar]:bg-muted [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-primary"
            />
          </div>
        )}

        {/* Expanded view */}
        {!isCollapsed && (
          <div className="px-3 py-2">
            {hasError ? (
              // Error state
              <div className="flex items-center justify-between">
                <span className="text-sm text-destructive">
                  {t("onboarding.downloadModel.error")}
                </span>
                <button
                  onClick={handleRetry}
                  disabled={isRetrying}
                  className="flex items-center gap-1 text-sm text-primary hover:text-primary/80 disabled:opacity-50"
                >
                  <RefreshCw className={`h-4 w-4 ${isRetrying ? "animate-spin" : ""}`} />
                  {t("onboarding.downloadModel.retry")}
                </button>
              </div>
            ) : isComplete ? (
              // Complete state - show success message
              <div className="flex items-center gap-2 text-green-600 dark:text-green-400">
                <Check className="h-4 w-4" />
                <span className="text-sm font-medium">
                  {t("onboarding.downloadModel.complete")}
                </span>
              </div>
            ) : isExtracting ? (
              // Extracting state - just show model name, no progress bar
              <>
                {modelName && (
                  <p className="text-sm text-muted-foreground">
                    {modelName}
                  </p>
                )}
              </>
            ) : (
              // Downloading state
              <>
                {modelName && (
                  <p className="text-sm text-muted-foreground mb-2">
                    {modelName}
                  </p>
                )}
                
                {downloadProgress && (
                  <div className="flex items-center gap-3 mb-1">
                    <progress
                      value={percentage}
                      max={100}
                      className="flex-1 h-2 [&::-webkit-progress-bar]:rounded-full [&::-webkit-progress-bar]:bg-muted [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-primary"
                    />
                    {downloadStats && downloadStats.speed > 0 && (
                      <span className="text-xs text-muted-foreground tabular-nums min-w-[60px] text-right">
                        {downloadStats.speed.toFixed(1)} MB/s
                      </span>
                    )}
                  </div>
                )}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default ModelDownloadProgress;
