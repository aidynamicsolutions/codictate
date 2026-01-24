import React, { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Download, Check, Loader2, Info, WifiOff } from "lucide-react";
import { Button } from "@/components/shared/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import OnboardingLayout from "./OnboardingLayout";
import { commands, type ModelInfo } from "@/bindings";
import { listen } from "@tauri-apps/api/event";
import {
  getTranslatedModelName,
  getTranslatedModelDescription,
} from "@/lib/utils/modelTranslation";
import { formatModelSize } from "@/lib/utils/format";

// Download states
type DownloadState = "idle" | "starting" | "downloading" | "extracting" | "complete" | "error";

interface DownloadProgress {
  model_id: string;
  downloaded: number;
  total: number;
  percentage: number;
}

interface DownloadStats {
  startTime: number;
  lastUpdate: number;
  totalDownloaded: number;
  speed: number;
}

interface ModelDownloadStepProps {
  onContinue: () => void;
  onBack?: () => void;
}

export const ModelDownloadStep: React.FC<ModelDownloadStepProps> = ({
  onContinue,
  onBack,
}) => {
  const { t } = useTranslation();
  
  // Model state
  const [recommendedModel, setRecommendedModel] = useState<ModelInfo | null>(null);
  const [downloadState, setDownloadState] = useState<DownloadState>("idle");
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadStats, setDownloadStats] = useState<DownloadStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isModelAlreadyDownloaded, setIsModelAlreadyDownloaded] = useState(false);
  
  // Refs for speed calculation and model tracking
  const lastProgressRef = useRef<{ downloaded: number; time: number } | null>(null);
  const speedSamplesRef = useRef<number[]>([]);
  const startTimeRef = useRef<number | null>(null);
  const recommendedModelRef = useRef<ModelInfo | null>(null);

  // Fetch recommended model on mount
  useEffect(() => {
    const fetchRecommendedModel = async () => {
      try {
        // Get the recommended model ID
        const recommendedResult = await commands.getRecommendedFirstModel();
        if (recommendedResult.status !== "ok") {
          console.error("Failed to get recommended model:", recommendedResult.error);
          return;
        }
        const modelId = recommendedResult.data;

        // Get model info
        const modelInfoResult = await commands.getModelInfo(modelId);
        if (modelInfoResult.status === "ok" && modelInfoResult.data) {
          setRecommendedModel(modelInfoResult.data);
          recommendedModelRef.current = modelInfoResult.data;
          
          // Check if already downloaded
          if (modelInfoResult.data.is_downloaded) {
            setIsModelAlreadyDownloaded(true);
            setDownloadState("complete");
          }
        }
      } catch (err) {
        console.error("Error fetching recommended model:", err);
      }
    };

    fetchRecommendedModel();
  }, []);

  // Listen to download events
  // Listen to download events - use refs to avoid stale closures (same pattern as ModelSelector.tsx)
  useEffect(() => {
    const progressUnlisten = listen<DownloadProgress>(
      "model-download-progress",
      (event) => {
        const progress = event.payload;
        const currentModel = recommendedModelRef.current;
        if (currentModel && progress.model_id === currentModel.id) {
          setDownloadProgress(progress);
          setDownloadState("downloading");

          // Calculate speed
          const now = Date.now();
          if (lastProgressRef.current) {
            const timeDiff = (now - lastProgressRef.current.time) / 1000; // seconds
            const bytesDiff = progress.downloaded - lastProgressRef.current.downloaded;
            if (timeDiff > 0) {
              const speed = bytesDiff / timeDiff / (1024 * 1024); // MB/s
              speedSamplesRef.current.push(speed);
              // Keep last 5 samples for smoothing
              if (speedSamplesRef.current.length > 5) {
                speedSamplesRef.current.shift();
              }
              const avgSpeed =
                speedSamplesRef.current.reduce((a, b) => a + b, 0) /
                speedSamplesRef.current.length;
              
              // Use ref for startTime to avoid stale closure
              if (!startTimeRef.current) {
                startTimeRef.current = now;
              }
              
              setDownloadStats({
                startTime: startTimeRef.current,
                lastUpdate: now,
                totalDownloaded: progress.downloaded,
                speed: avgSpeed,
              });
            }
          }
          lastProgressRef.current = { downloaded: progress.downloaded, time: now };
        }
      }
    );

    const completeUnlisten = listen<string>("model-download-complete", (event) => {
      const currentModel = recommendedModelRef.current;
      if (currentModel && event.payload === currentModel.id) {
        // Download finished, extraction will start automatically
        // Keep the download state until extraction starts
        setDownloadProgress(null);
      }
    });

    // Listen for extraction started event
    const extractionStartedUnlisten = listen<string>(
      "model-extraction-started",
      (event) => {
        const currentModel = recommendedModelRef.current;
        if (currentModel && event.payload === currentModel.id) {
          setDownloadState("extracting");
        }
      }
    );

    const extractionCompletedUnlisten = listen<string>(
      "model-extraction-completed",
      (event) => {
        const currentModel = recommendedModelRef.current;
        if (currentModel && event.payload === currentModel.id) {
          setDownloadState("complete");
        }
      }
    );

    const extractionFailedUnlisten = listen<{ model_id: string; error: string }>(
      "model-extraction-failed",
      (event) => {
        const currentModel = recommendedModelRef.current;
        if (currentModel && event.payload.model_id === currentModel.id) {
          setDownloadState("error");
          setError(event.payload.error);
        }
      }
    );

    return () => {
      progressUnlisten.then((fn) => fn());
      completeUnlisten.then((fn) => fn());
      extractionStartedUnlisten.then((fn) => fn());
      extractionCompletedUnlisten.then((fn) => fn());
      extractionFailedUnlisten.then((fn) => fn());
    };
  }, []);

  // Handle download initiation
  const handleDownload = useCallback(async () => {
    if (!recommendedModel) return;

    setDownloadState("starting");
    setError(null);
    lastProgressRef.current = null;
    speedSamplesRef.current = [];

    try {
      const result = await commands.downloadModel(recommendedModel.id);
      if (result.status !== "ok") {
        setDownloadState("error");
        setError(result.error);
      }
      // Download started - state will update via events
    } catch (err) {
      setDownloadState("error");
      setError(String(err));
    }
  }, [recommendedModel]);

  // Handle retry
  const handleRetry = useCallback(() => {
    handleDownload();
  }, [handleDownload]);

  // Get dynamic progress message based on percentage
  const getProgressMessage = useCallback(() => {
    if (!downloadProgress) return t("onboarding.downloadModel.progressMessages.starting");
    
    const percentage = downloadProgress.percentage;
    if (percentage < 10) {
      return t("onboarding.downloadModel.progressMessages.starting");
    } else if (percentage < 50) {
      return t("onboarding.downloadModel.progressMessages.inProgress");
    } else if (percentage < 90) {
      return t("onboarding.downloadModel.progressMessages.almostDone");
    } else {
      return t("onboarding.downloadModel.progressMessages.finishing");
    }
  }, [downloadProgress, t]);

  // Estimate time remaining
  const getEstimatedTime = useCallback(() => {
    if (!downloadProgress || !downloadStats || downloadStats.speed <= 0) return null;
    
    const remainingBytes = downloadProgress.total - downloadProgress.downloaded;
    const remainingSeconds = remainingBytes / (downloadStats.speed * 1024 * 1024);
    const minutes = Math.ceil(remainingSeconds / 60);
    
    if (minutes < 1) return null;
    return t("onboarding.downloadModel.estimatedTime", { minutes });
  }, [downloadProgress, downloadStats, t]);

  // Can continue if download has started, extracting, or is complete
  const canContinue = downloadState === "downloading" || downloadState === "extracting" || downloadState === "complete";
  const showContinuePulse = downloadState === "downloading" || downloadState === "extracting";

  // Get translated model info
  const modelName = recommendedModel
    ? getTranslatedModelName(recommendedModel, t)
    : "";
  const modelDescription = recommendedModel
    ? getTranslatedModelDescription(recommendedModel, t)
    : "";

  return (
    <OnboardingLayout
      currentStep="downloadModel"
      leftContent={
        <div className="flex flex-col h-full">
          {/* Back button - positioned at top */}
          {onBack && (
            <button
              type="button"
              onClick={onBack}
              className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors w-fit mb-auto"
            >
              <ArrowLeft className="h-4 w-4" />
              {t("onboarding.downloadModel.back")}
            </button>
          )}

          {/* Content centered vertically */}
          <div className="flex flex-col gap-6 my-auto">
            <div className="flex flex-col gap-2 mb-4">
              <h1 className="text-3xl font-semibold tracking-tight text-foreground lg:text-4xl max-w-[420px]">
                {t("onboarding.downloadModel.title")}
              </h1>
              <p className="text-muted-foreground max-w-[380px]">
                {t("onboarding.downloadModel.subtitle", { appName: t("appName") })}
              </p>
            </div>

            {/* Model info card */}
            {recommendedModel && (
              <div className="rounded-lg border border-border bg-background p-4">
                <p className="text-sm text-muted-foreground mb-3">
                  {t("onboarding.downloadModel.modelInfo")}
                </p>
                
                <div className="flex items-start justify-between">
                  <div className="flex flex-col gap-1">
                    <span className="font-medium text-foreground text-lg">
                      {modelName}
                    </span>
                    <span className="text-sm text-muted-foreground">
                      {modelDescription}
                    </span>
                    <div className="flex items-center gap-2 mt-2">
                      <Download className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm text-muted-foreground">
                        {formatModelSize(recommendedModel.size_mb)}
                      </span>
                    </div>
                  </div>
                  
                  {/* Speed score indicator */}
                  <div className="flex flex-col items-end gap-1">
                    <span className="text-xs text-muted-foreground">
                      {t("onboarding.modelCard.speed")}
                    </span>
                    <div className="w-16 h-2 bg-muted rounded-full overflow-hidden">
                      <div
                        className="h-full bg-primary rounded-full transition-all duration-300"
                        style={{ width: `${recommendedModel.speed_score * 100}%` }}
                      />
                    </div>
                  </div>
                </div>

                {/* Default model note */}
                <p className="text-xs text-muted-foreground mt-3">
                  {t("onboarding.downloadModel.defaultModelNote", { modelName })}
                </p>

                {/* Download progress */}
                {downloadState === "downloading" && downloadProgress && (
                  <div className="mt-4 pt-4 border-t border-border">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-medium text-foreground">
                        {t("onboarding.downloadModel.downloading")} {Math.round(downloadProgress.percentage)}%
                      </span>
                      {downloadStats && downloadStats.speed > 0 && (
                        <span className="text-xs text-muted-foreground tabular-nums">
                          {downloadStats.speed.toFixed(1)} MB/s
                        </span>
                      )}
                    </div>
                    <progress
                      value={downloadProgress.percentage}
                      max={100}
                      className="w-full h-2 [&::-webkit-progress-bar]:rounded-full [&::-webkit-progress-bar]:bg-muted [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-primary"
                    />
                    {getEstimatedTime() && (
                      <p className="text-xs text-muted-foreground mt-2">
                        {getEstimatedTime()}
                      </p>
                    )}
                  </div>
                )}

                {/* Extracting state */}
                {downloadState === "extracting" && (
                  <div className="mt-4 pt-4 border-t border-border">
                    <div className="flex items-center gap-2">
                      <Loader2 className="h-4 w-4 animate-spin text-primary" />
                      <span className="text-sm font-medium text-foreground">
                        {t("modelSelector.extractingGeneric")}
                      </span>
                    </div>
                  </div>
                )}

                {/* Complete state */}
                {downloadState === "complete" && (
                  <div className="mt-4 pt-4 border-t border-border">
                    <div className="flex items-center gap-2 text-green-600 dark:text-green-400">
                      <Check className="h-5 w-5" />
                      <span className="text-sm font-medium">
                        {t("onboarding.downloadModel.complete")}
                      </span>
                    </div>
                  </div>
                )}

                {/* Error state */}
                {downloadState === "error" && (
                  <div className="mt-4 pt-4 border-t border-border">
                    <p className="text-sm text-destructive mb-3">
                      {t("onboarding.downloadModel.error")}
                    </p>
                    <Button onClick={handleRetry} variant="outline" size="sm">
                      {t("onboarding.downloadModel.retry")}
                    </Button>
                  </div>
                )}
              </div>
            )}

            {/* Download button */}
            {downloadState === "idle" && !isModelAlreadyDownloaded && (
              <Button
                onClick={handleDownload}
                size="lg"
                className="w-fit"
                disabled={!recommendedModel}
              >
                <Download className="mr-2 h-5 w-5" />
                {t("onboarding.downloadModel.downloadButton")}
              </Button>
            )}

            {/* Starting state */}
            {downloadState === "starting" && (
              <Button size="lg" className="w-fit" disabled>
                <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                {t("onboarding.downloadModel.downloading")}
              </Button>
            )}

            {/* Dynamic progress message */}
            {downloadState === "downloading" && (
              <div className="flex items-start gap-2">
                <p className="text-sm text-muted-foreground">
                  {getProgressMessage()}
                </p>
              </div>
            )}

            {/* Background download hint with tooltip */}
            {canContinue && downloadState !== "complete" && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span>{t("onboarding.downloadModel.backgroundHint")}</span>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      className="rounded-full p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
                    >
                      <Info className="h-4 w-4" />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side="right" className="max-w-xs">
                    <p>{t("onboarding.downloadModel.continueTooltip")}</p>
                  </TooltipContent>
                </Tooltip>
              </div>
            )}
          </div>

          {/* Continue button at bottom */}
          <Button
            onClick={onContinue}
            size="lg"
            className={`mt-auto w-fit ${showContinuePulse ? "animate-pulse-gentle" : ""}`}
            disabled={!canContinue && !isModelAlreadyDownloaded}
          >
            {t("onboarding.downloadModel.continue")}
          </Button>
        </div>
      }
      rightContent={
        <div className="flex flex-col items-center justify-center h-full gap-6">
          {/* Offline Privacy Badge Illustration */}
          <div className="relative flex items-center justify-center">
            {/* Shield SVG Background */}
            <svg
              width="180"
              height="220"
              viewBox="0 0 180 220"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
              className="drop-shadow-xl"
            >
              {/* Shield/Badge Shape with Gradient */}
              <defs>
                <linearGradient id="shieldGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="#e63946" />
                  <stop offset="50%" stopColor="#d62839" />
                  <stop offset="100%" stopColor="#c1121f" />
                </linearGradient>
                <filter id="shieldShadow" x="-20%" y="-10%" width="140%" height="130%">
                  <feDropShadow dx="0" dy="8" stdDeviation="12" floodColor="#c1121f" floodOpacity="0.35"/>
                </filter>
              </defs>
              
              {/* Main Shield Shape */}
              <path
                d="M90 15 L160 42 C165 44 168 50 168 56 L168 105 C168 150 130 180 90 205 C50 180 12 150 12 105 L12 56 C12 50 15 44 20 42 L90 15Z"
                fill="url(#shieldGradient)"
                filter="url(#shieldShadow)"
              />
            </svg>
            
            {/* Lucide WifiOff Icon - positioned over shield center */}
            <div className="absolute inset-0 flex items-center justify-center" style={{ marginTop: '-12px' }}>
              <WifiOff 
                className="text-white drop-shadow-md" 
                size={72} 
                strokeWidth={2}
              />
            </div>
          </div>
          
          {/* Privacy note */}
          <div className="text-center max-w-[280px]">
            <p className="text-sm text-zinc-600 font-medium">
              {t("onboarding.downloadModel.privacyNote")}
            </p>
          </div>
        </div>
      }
    />
  );
};

export default ModelDownloadStep;
