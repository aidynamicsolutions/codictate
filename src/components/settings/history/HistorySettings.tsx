import React, { useState, useEffect, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AudioPlayer } from "../../ui/AudioPlayer";
import { Button } from "@/components/shared/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/shared/ui/card";
import { ScrollArea } from "@/components/shared/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { Skeleton } from "@/components/shared/ui/skeleton";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { Copy, Star, Check, Trash2, FolderOpen, Loader2 } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { platform } from "@tauri-apps/plugin-os";
import { readFile } from "@tauri-apps/plugin-fs";
import { commands, type HistoryEntry } from "@/bindings";
import { formatDate } from "@/utils/dateFormat";
import { logError, logInfo } from "@/utils/logging";

const IS_LINUX = platform() === "linux";

interface OpenRecordingsButtonProps {
  onClick: () => void;
  label: string;
}

const OpenRecordingsButton: React.FC<OpenRecordingsButtonProps> = ({
  onClick,
  label,
}) => (
  <Button
    onClick={onClick}
    variant="outline"
    size="sm"
    className="flex items-center gap-2 h-8 text-xs font-medium"
    title={label}
  >
    <FolderOpen className="w-3.5 h-3.5" />
    <span>{label}</span>
  </Button>
);

export const HistorySettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [showClearDialog, setShowClearDialog] = useState(false);
  const [isClearing, setIsClearing] = useState(false);

  const loadHistoryEntries = useCallback(async () => {
    try {
      const result = await commands.getHistoryEntries();
      if (result.status === "ok") {
        setHistoryEntries(result.data);
      }
    } catch (error) {
      logError(`Failed to load history entries: ${error}`, "fe-history");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadHistoryEntries();

    const setupListener = async () => {
      const unlisten = await listen("history-updated", () => {
        logInfo("History updated, reloading entries...", "fe-history");
        loadHistoryEntries();
      });
      return unlisten;
    };

    let unlistenPromise = setupListener();

    return () => {
      unlistenPromise.then((unlisten) => {
        if (unlisten) unlisten();
      });
    };
  }, [loadHistoryEntries]);

  const toggleSaved = async (id: number) => {
    try {
      await commands.toggleHistoryEntrySaved(id);
    } catch (error) {
      logError(`Failed to toggle saved status: ${error}`, "fe-history");
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      logError(`Failed to copy to clipboard: ${error}`, "fe-history");
    }
  };

  const getAudioUrl = async (fileName: string) => {
    try {
      const result = await commands.getAudioFilePath(fileName);
      if (result.status === "ok") {
        if (IS_LINUX) {
          const fileData = await readFile(result.data);
          const blob = new Blob([fileData], { type: "audio/wav" });
          return URL.createObjectURL(blob);
        }
        return convertFileSrc(result.data, "asset");
      }
      return null;
    } catch (error) {
      logError(`Failed to get audio file path: ${error}`, "fe-history");
      return null;
    }
  };

  const deleteAudioEntry = async (id: number) => {
    try {
      await commands.deleteHistoryEntry(id);
    } catch (error) {
      logError(`Failed to delete audio entry: ${error}`, "fe-history");
      throw error;
    }
  };

  const clearAllHistory = async () => {
    setIsClearing(true);
    try {
      logInfo("Clearing all history entries...", "fe-history");
      await commands.clearAllHistory();
      logInfo("Successfully cleared all history", "fe-history");
      setShowClearDialog(false);
    } catch (error) {
      logError(`Failed to clear all history: ${error}`, "fe-history");
    } finally {
      setIsClearing(false);
    }
  };

  const openRecordingsFolder = async () => {
    try {
      await commands.openRecordingsFolder();
    } catch (error) {
      logError(`Failed to open recordings folder: ${error}`, "fe-history");
    }
  };

  const groupedEntries = useMemo(() => {
    const groups: { [key: string]: HistoryEntry[] } = {};
    historyEntries.forEach((entry) => {
      // Create a localized date string for grouping
      const dateKey = formatDate(String(entry.timestamp), i18n.language);
      if (!groups[dateKey]) {
        groups[dateKey] = [];
      }
      groups[dateKey].push(entry);
    });
    return groups;
  }, [historyEntries, i18n.language]);

  // Sort dates descending (newest first)
  const sortedDates = Object.keys(groupedEntries).sort((a, b) => {
    // We can pick the first entry of each group to compare timestamps since they are grouped by date
    const timestampA = groupedEntries[a][0].timestamp;
    const timestampB = groupedEntries[b][0].timestamp;
    return timestampB - timestampA;
  });

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <Card className="w-full h-full animate-in fade-in slide-in-from-bottom-2 duration-500 bg-card/60 backdrop-blur-sm border-border/60 hover:border-border/80 transition-colors">
        <CardHeader className="pb-3 flex flex-row items-center justify-between space-y-0">
          <CardTitle className="text-sm font-semibold uppercase tracking-wide text-primary font-heading">
            {t("settings.history.title")}
          </CardTitle>
          <div className="flex items-center gap-2">
            {historyEntries.length > 0 && (
              <Button
                onClick={() => setShowClearDialog(true)}
                variant="outline"
                size="sm"
                className="flex items-center gap-2 h-8 text-xs font-medium text-destructive hover:bg-destructive/10 hover:text-destructive border-destructive/30"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>{t("settings.history.clearAll")}</span>
              </Button>
            )}
            <OpenRecordingsButton
              onClick={openRecordingsFolder}
              label={t("settings.history.openFolder")}
            />
          </div>
        </CardHeader>
        <CardContent className="p-0">
          <div className="min-h-[300px]">
            {loading ? (
              <div className="p-6 space-y-6">
                {[1, 2].map((i) => (
                  <div key={i} className="space-y-4">
                    <Skeleton className="h-4 w-32" />
                    <div className="space-y-3">
                      <Skeleton className="h-20 w-full" />
                      <Skeleton className="h-20 w-full" />
                    </div>
                  </div>
                ))}
              </div>
            ) : historyEntries.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-20 text-center px-4">
                <div className="bg-muted/50 p-4 rounded-full mb-4">
                  <FolderOpen className="w-8 h-8 text-muted-foreground/50" />
                </div>
                <p className="text-muted-foreground font-medium mb-1">
                  {t("settings.history.empty")}
                </p>
                <p className="text-xs text-muted-foreground/70 max-w-xs">
                  {t("settings.history.emptyDescription")}
                </p>
              </div>
            ) : (
              <ScrollArea className="h-[600px] w-full">
                <div className="pb-8 px-6">
                  <TooltipProvider delayDuration={300}>
                    {sortedDates.map((date, groupIndex) => (
                      <div
                        key={date}
                        className="mb-8 last:mb-0 animate-in slide-in-from-bottom-2 fade-in duration-500 fill-mode-both"
                        style={{ animationDelay: `${groupIndex * 100}ms` }}
                      >
                        <div className="sticky top-0 z-10 bg-card/95 backdrop-blur supports-[backdrop-filter]:bg-card/75 py-3 mb-2 border-b border-border/40">
                          <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                            {date}
                          </h3>
                        </div>
                        <div className="space-y-1">
                          {groupedEntries[date].map((entry) => (
                            <TimelineItem
                              key={entry.id}
                              entry={entry}
                              onToggleSaved={() => toggleSaved(entry.id)}
                              onCopyText={() => copyToClipboard(entry.transcription_text)}
                              getAudioUrl={getAudioUrl}
                              deleteAudio={deleteAudioEntry}
                            />
                          ))}
                        </div>
                      </div>
                    ))}
                  </TooltipProvider>
                </div>
              </ScrollArea>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Clear All Confirmation Dialog */}
      <Dialog open={showClearDialog} onOpenChange={setShowClearDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("settings.history.clearAllConfirmTitle")}</DialogTitle>
            <DialogDescription>
              {t("settings.history.clearAllConfirmDescription", { count: historyEntries.length })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowClearDialog(false)}
              disabled={isClearing}
            >
              {t("common.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={clearAllHistory}
              disabled={isClearing}
            >
              {isClearing ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  {t("common.loading")}
                </>
              ) : (
                t("settings.history.clearAllConfirmButton")
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

interface TimelineItemProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => void;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
}

const TimelineItem: React.FC<TimelineItemProps> = ({
  entry,
  onToggleSaved,
  onCopyText,
  getAudioUrl,
  deleteAudio,
}) => {
  const { t, i18n } = useTranslation();
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [showCopied, setShowCopied] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);

  useEffect(() => {
    let cancelled = false;
    let urlToRevoke: string | null = null;

    const loadAudio = async () => {
      const url = await getAudioUrl(entry.file_name);
      if (!cancelled) {
        urlToRevoke = url;
        setAudioUrl(url);
      } else if (url?.startsWith("blob:")) {
        URL.revokeObjectURL(url);
      }
    };

    loadAudio();

    return () => {
      cancelled = true;
      if (urlToRevoke?.startsWith("blob:")) {
        URL.revokeObjectURL(urlToRevoke);
      }
    };
  }, [entry.file_name, getAudioUrl]);

  const handleCopyText = () => {
    onCopyText();
    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  const handleDeleteEntry = async () => {
    if (isDeleting) return;
    setIsDeleting(true);
    try {
      await deleteAudio(entry.id);
    } catch (error) {
      logError(`Failed to delete entry: ${error}`, "fe-history");
      setIsDeleting(false);
    }
  };

  // Format time (e.g., "06:45 PM")
  const formattedTime = new Intl.DateTimeFormat(i18n.language, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(entry.timestamp * 1000));

  return (
    <div className="group flex flex-row items-start py-4 px-2 rounded-md hover:bg-muted/30 transition-colors gap-4 border-b border-border/20 last:border-b-0">
      {/* Time Column */}
      <div className="w-20 shrink-0 pt-0.5">
        <span className="text-xs font-medium text-muted-foreground/80 tabular-nums">
          {formattedTime}
        </span>
      </div>

      {/* Content Column */}
      <div className="flex-1 min-w-0 space-y-3">
        <p className="text-sm leading-relaxed text-foreground/90 select-text cursor-text break-words">
          {entry.transcription_text}
        </p>
        {audioUrl && (
            <AudioPlayer src={audioUrl} className="w-full max-w-md" />
        )}
      </div>

      {/* Actions Column - Visible on Group Hover */}
      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity ml-2 shrink-0 self-start">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleCopyText}
            >
              {showCopied ? (
                <Check className="w-4 h-4 text-green-500" />
              ) : (
                <Copy className="w-4 h-4 text-muted-foreground" />
              )}
              <span className="sr-only">
                {t("settings.history.copyToClipboard")}
              </span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>
              {showCopied
                ? t("common.copied")
                : t("settings.history.copyToClipboard")}
            </p>
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className={`h-8 w-8 ${
                entry.saved
                  ? "text-yellow-500 hover:text-yellow-600"
                  : "text-muted-foreground"
              }`}
              onClick={onToggleSaved}
            >
              <Star
                className="w-4 h-4"
                fill={entry.saved ? "currentColor" : "none"}
              />
              <span className="sr-only">
                {entry.saved
                  ? t("settings.history.unsave")
                  : t("settings.history.save")}
              </span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>
              {entry.saved
                ? t("settings.history.unsave")
                : t("settings.history.save")}
            </p>
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 text-muted-foreground hover:text-destructive hover:bg-destructive/10"
              onClick={handleDeleteEntry}
              disabled={isDeleting}
            >
              {isDeleting ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Trash2 className="w-4 h-4" />
              )}
              <span className="sr-only">{t("settings.history.delete")}</span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{t("settings.history.delete")}</p>
          </TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
};
