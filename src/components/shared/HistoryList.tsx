import React, { useState, useEffect, useMemo, useCallback, useRef } from "react";
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
import { GroupedVirtuoso, GroupedVirtuosoHandle } from "react-virtuoso";
import { convertFileSrc } from "@tauri-apps/api/core";

interface HistoryListProps {
  loading: boolean;
  historyEntries: HistoryEntry[];
  sortedDates: string[];
  groupedEntries: { [key: string]: HistoryEntry[] };
  onToggleSaved: (id: number) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
  className?: string; // Allow external styling (e.g., for height/scrolling)
  emptyMessage?: string;
  emptyDescription?: string;
  disableScrollArea?: boolean;
  searchQuery?: string;
  stickyTopOffset?: number | string;
  loadMore?: () => void;
  hasMore?: boolean;
  scrollContainer?: HTMLElement | null;
}

export const HistoryList: React.FC<HistoryListProps> = React.memo(({
  loading,
  historyEntries,
  sortedDates,
  groupedEntries,
  onToggleSaved,
  onDelete,
  className,
  emptyMessage,
  emptyDescription,
  disableScrollArea = false,
  searchQuery = "",
  stickyTopOffset = 0,
  loadMore,
  hasMore = false,
  scrollContainer,
}) => {
  const { t } = useTranslation();
  const [currentGroupIndex, setCurrentGroupIndex] = useState(0);
  const virtuosoRef = useRef<GroupedVirtuosoHandle>(null);

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      logError(`Failed to copy to clipboard: ${error}`, "fe-history");
    }
  };

  // Calculate group counts for Virtuoso
  const groupCounts = useMemo(() => {
    return sortedDates.map(date => groupedEntries[date]?.length || 0);
  }, [sortedDates, groupedEntries]);

  // Flatten entries for item access
  const flattenedEntries = useMemo(() => {
     return sortedDates.flatMap(date => groupedEntries[date] || []);
  }, [sortedDates, groupedEntries]);

  // Calculate cumulative item counts to determine which group an item belongs to
  const cumulativeCounts = useMemo(() => {
    let sum = 0;
    return groupCounts.map(count => {
      const result = sum;
      sum += count;
      return result;
    });
  }, [groupCounts]);

  // Determine which group a given item index belongs to
  const getGroupForItem = useCallback((itemIndex: number) => {
    for (let i = cumulativeCounts.length - 1; i >= 0; i--) {
      if (itemIndex >= cumulativeCounts[i]) {
        return i;
      }
    }
    return 0;
  }, [cumulativeCounts]);

  if (loading && historyEntries.length === 0) {
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

  if (historyEntries.length === 0 && !loading) {
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

  const renderItem = (index: number) => {
    const entry = flattenedEntries[index];
    if (!entry) return null;
    return (
        <div className="px-6 pb-1">
            <TimelineItem
                entry={entry}
                onToggleSaved={() => onToggleSaved(entry.id)}
                onCopyText={() => copyToClipboard(entry.transcription_text)}
                deleteAudio={onDelete}
                searchQuery={searchQuery}
            />
        </div>
    );
  }
  
  const renderGroup = (index: number) => {
      const date = sortedDates[index];
      // Hide the first group header when using customScrollParent since
      // the external sticky header already shows it
      if (scrollContainer && index === 0) {
        return <div className="h-0" />;
      }
      return (
        <div
          className="bg-card/95 backdrop-blur supports-[backdrop-filter]:bg-card/85 py-3 mb-2 border-b border-border/60 shadow-sm px-6"
        >
          <h3 className="text-xs font-bold text-primary/80 uppercase tracking-widest px-1">
            {date}
          </h3>
        </div>
      );
  }
  
  const Footer = () => {
    // Try to maintain consistent height to avoid jumps
    // py-4 (1rem top + 1rem bottom = 32px) + icon (24px) = 56px => h-14
    return (
        <div className="h-14 flex justify-center w-full items-center">
            {loading && hasMore && (
                <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
            )}
        </div>
    )
  }

  const topValue = typeof stickyTopOffset === 'number' ? `${stickyTopOffset}px` : stickyTopOffset;
  const currentDate = sortedDates[currentGroupIndex] || sortedDates[0];

  return (
    <TooltipProvider delayDuration={300}>
        {/* External sticky header - only shown when using customScrollParent */}
        {scrollContainer && sortedDates.length > 0 && (
          <div
            className="bg-card/95 backdrop-blur supports-[backdrop-filter]:bg-card/85 py-3 border-b border-border/60 shadow-sm px-6"
            style={{ position: 'sticky', top: topValue, zIndex: 15 }}
          >
            <h3 className="text-xs font-bold text-primary/80 uppercase tracking-widest px-1">
              {currentDate}
            </h3>
          </div>
        )}
        <GroupedVirtuoso
            ref={virtuosoRef}
            style={scrollContainer ? { height: 'auto' } : { height: '100%', flex: 1 }}
            customScrollParent={scrollContainer || undefined}
            groupCounts={groupCounts}
            groupContent={renderGroup}
            itemContent={renderItem}
            components={{
              Footer: Footer,
            }}
            rangeChanged={(range) => {
              // Update the current group based on the first visible item
              if (range.startIndex !== undefined) {
                const newGroupIndex = getGroupForItem(range.startIndex);
                if (newGroupIndex !== currentGroupIndex) {
                  setCurrentGroupIndex(newGroupIndex);
                }
              }
            }}
            endReached={() => {
                if (hasMore && !loading && loadMore) {
                    loadMore();
                }
            }}
            overscan={600}
        />
    </TooltipProvider>
  );
});

interface TimelineItemProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => void;
  deleteAudio: (id: number) => Promise<void>;
  searchQuery?: string;
}

const TimelineItem: React.FC<TimelineItemProps> = React.memo(({
  entry,
  onToggleSaved,
  onCopyText,
  deleteAudio,
  searchQuery = "",
}) => {
  const { t, i18n } = useTranslation();
  const [showCopied, setShowCopied] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  
  // Platform-specific audio loading strategy
  const audioProps = useMemo(() => {
    if (!entry.file_path) return {};
    
    // Simple heuristic for Linux - reliable enough for browser environment
    const isLinux = navigator.userAgent.includes("Linux");
    
    if (isLinux) {
      return {
        onLoadRequest: async () => {
           try {
              // Dynamic import to avoid issues on non-Tauri envs (though we are in Tauri)
              const { readFile } = await import("@tauri-apps/plugin-fs");
              const fileData = await readFile(entry.file_path);
              const blob = new Blob([fileData], { type: "audio/wav" });
              return URL.createObjectURL(blob);
           } catch (e) {
               logError(`Failed to load audio on Linux: ${e}`, "fe-history");
               return null;
           }
        }
      };
    }
    
    // Mac/Windows: Use direct src via convertFileSrc for instant playback
    return {
      src: convertFileSrc(entry.file_path)
    };
  }, [entry.file_path]);

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
        {(audioProps.src || audioProps.onLoadRequest) && (
          <AudioPlayer {...audioProps} className="w-full max-w-md" />
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
