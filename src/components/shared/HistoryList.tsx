import React, { useState, useEffect } from "react";
import { ScrollArea } from "@/components/shared/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { Button } from "@/components/shared/ui/button";
import { Skeleton } from "@/components/shared/ui/skeleton";
import { FolderOpen, Copy, Star, Check, Trash2, Loader2 } from "lucide-react";
import { AudioPlayer } from "@/components/ui/AudioPlayer";
import { useTranslation } from "react-i18next";
import { type HistoryEntry } from "@/bindings";
import { logError } from "@/utils/logging";

interface HistoryListProps {
  loading: boolean;
  historyEntries: HistoryEntry[];
  sortedDates: string[];
  groupedEntries: { [key: string]: HistoryEntry[] };
  onToggleSaved: (id: number) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  className?: string; // Allow external styling (e.g., for height/scrolling)
  emptyMessage?: string;
  emptyDescription?: string;
  disableScrollArea?: boolean;
  searchQuery?: string;
  stickyTopOffset?: number | string;
}

export const HistoryList: React.FC<HistoryListProps> = React.memo(({
  loading,
  historyEntries,
  sortedDates,
  groupedEntries,
  onToggleSaved,
  onDelete,
  getAudioUrl,
  className,
  emptyMessage,
  emptyDescription,
  disableScrollArea = false,
  searchQuery = "",
  stickyTopOffset = 0,
}) => {
  const { t } = useTranslation();

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      logError(`Failed to copy to clipboard: ${error}`, "fe-history");
    }
  };

  if (loading) {
    return (
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
    );
  }

  if (historyEntries.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center py-20 text-center px-4">
        <div className="bg-muted/50 p-4 rounded-full mb-4">
          <FolderOpen className="w-8 h-8 text-muted-foreground/50" />
        </div>
        <p className="text-muted-foreground font-medium mb-1">
          {emptyMessage || t("settings.history.empty")}
        </p>
        <p className="text-xs text-muted-foreground/70 max-w-xs">
          {emptyDescription || t("settings.history.emptyDescription")}
        </p>
      </div>
    );
  }

  const Content = (
    <div className="pb-8 px-6">
      <TooltipProvider delayDuration={300}>
        {sortedDates.map((date, groupIndex) => (
          <div
            key={date}
            className="mb-8 last:mb-0 animate-in slide-in-from-bottom-2 fade-in duration-500 fill-mode-both"
            style={{ animationDelay: `${groupIndex * 100}ms` }}
          >
            <div
              className="sticky z-10 bg-card/95 backdrop-blur supports-[backdrop-filter]:bg-card/85 py-3 mb-2 border-b border-border/60 shadow-sm -mx-6 px-6"
              style={{ top: stickyTopOffset }}
            >
              <h3 className="text-xs font-bold text-primary/80 uppercase tracking-widest px-1">
                {date}
              </h3>
            </div>
            <div className="space-y-1">
              {groupedEntries[date].map((entry) => (
                <TimelineItem
                  key={entry.id}
                  entry={entry}
                  onToggleSaved={() => onToggleSaved(entry.id)}
                  onCopyText={() => copyToClipboard(entry.transcription_text)}
                  getAudioUrl={getAudioUrl}
                  deleteAudio={onDelete}
                  searchQuery={searchQuery}
                />
              ))}
            </div>
          </div>
        ))}
      </TooltipProvider>
    </div>
  );

  if (disableScrollArea) {
    return <div className={`flex-1 w-full ${className}`}>{Content}</div>;
  }

  return (
    <ScrollArea className={`flex-1 w-full min-h-0 ${className}`}>
      {Content}
    </ScrollArea>
  );
});

interface TimelineItemProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => void;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
  searchQuery?: string;
}

const TimelineItem: React.FC<TimelineItemProps> = React.memo(({
  entry,
  onToggleSaved,
  onCopyText,
  getAudioUrl,
  deleteAudio,
  searchQuery = "",
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

  const highlightText = (text: string, query: string) => {
    if (!query || query.length === 0) return text;

    const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const parts = text.split(new RegExp(`(${escapedQuery})`, "gi"));

    return parts.map((part, i) =>
      part.toLowerCase() === query.toLowerCase() ? (
        <span
          key={i}
          className="bg-yellow-500/30 text-foreground rounded-[2px] px-0.5 -mx-0.5 font-medium"
        >
          {part}
        </span>
      ) : (
        part
      )
    );
  };

  return (
    <div className="group flex flex-row items-start py-4 px-3 rounded-lg hover:bg-accent/40 transition-all duration-200 gap-4 border border-transparent hover:border-border/50 mb-1">
      {/* Time Column */}
      <div className="w-20 shrink-0 pt-0.5">
        <span className="text-xs font-medium text-muted-foreground/80 tabular-nums">
          {formattedTime}
        </span>
      </div>

      {/* Content Column */}
      <div className="flex-1 min-w-0 space-y-3">
        <p className="text-sm leading-relaxed text-foreground/90 select-text cursor-text break-words">
          {highlightText(entry.transcription_text, searchQuery)}
        </p>
        {audioUrl && (
          <AudioPlayer src={audioUrl} className="w-full max-w-md" />
        )}
      </div>

      {/* Actions Column - Visible on Group Hover */}
      <div className="flex items-center gap-1 ml-2 shrink-0 self-start text-muted-foreground/40 hover:text-muted-foreground transition-colors duration-200">
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
                <Copy className="w-4 h-4" />
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
                  : ""
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
              className="h-8 w-8 hover:text-destructive hover:bg-destructive/10"
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
});
