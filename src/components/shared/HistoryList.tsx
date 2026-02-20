import React, {
  useState,
  useEffect,
  useMemo,
  useCallback,
  useRef,
} from "react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { Button } from "@/components/shared/ui/button";
import { Skeleton } from "@/components/shared/ui/skeleton";
import {
  FolderOpen,
  Copy,
  Star,
  Check,
  Trash2,
  Loader2,
  Sparkles,
  FileText,
} from "lucide-react";
import { AudioPlayer } from "@/components/ui/AudioPlayer";
import { useTranslation } from "react-i18next";
import { type HistoryEntry } from "@/bindings";
import { logError, logInfo } from "@/utils/logging";
import { GroupedVirtuoso, GroupedVirtuosoHandle } from "react-virtuoso";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useSettings } from "@/hooks/useSettings";
import {
  dictionaryEntryIdentity,
  normalizeAliases,
} from "@/utils/dictionaryUtils";
import {
  type AliasSuggestion,
  suggestAliasFromTranscript,
} from "@/utils/dictionaryAliasSuggestion";
import { toast } from "sonner";
import { useSettingsStore } from "@/stores/settingsStore";
import {
  getHistoryCopyText,
  getHistoryPrimaryText,
  getHistoryRawText,
  isRawOnlyHistoryMatch,
  shouldShowOriginalTranscript,
} from "@/components/shared/historyDisplayUtils";

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
  onNavigate?: (section: string) => void;
}

export const HistoryList: React.FC<HistoryListProps> = React.memo(
  ({
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
    onNavigate,
  }) => {
    const { t } = useTranslation();
    const { settings, updateSetting } = useSettings();
    const [currentGroupIndex, setCurrentGroupIndex] = useState(0);
    const [addingAliasByEntryId, setAddingAliasByEntryId] = useState<
      Record<number, boolean>
    >({});
    const virtuosoRef = useRef<GroupedVirtuosoHandle>(null);
    const aliasApplyInFlightRef = useRef(new Set<number>());
    const dictionaryEntries = useMemo(
      () => settings?.dictionary ?? [],
      [settings?.dictionary],
    );

    const copyToClipboard = async (text: string) => {
      try {
        await navigator.clipboard.writeText(text);
      } catch (error) {
        logError(`Failed to copy to clipboard: ${error}`, "fe-history");
      }
    };

    // Calculate group counts for Virtuoso
    const groupCounts = useMemo(() => {
      return sortedDates.map((date) => groupedEntries[date]?.length || 0);
    }, [sortedDates, groupedEntries]);

    // Flatten entries for item access
    const flattenedEntries = useMemo(() => {
      return sortedDates.flatMap((date) => groupedEntries[date] || []);
    }, [sortedDates, groupedEntries]);

    const aliasSuggestionByHistoryId = useMemo(() => {
      const suggestions = new Map<number, AliasSuggestion>();
      if (dictionaryEntries.length === 0) {
        return suggestions;
      }

      for (const entry of flattenedEntries) {
        const suggestion = suggestAliasFromTranscript(
          getHistoryRawText(entry),
          dictionaryEntries,
        );
        if (suggestion) {
          suggestions.set(entry.id, suggestion);
        }
      }
      return suggestions;
    }, [dictionaryEntries, flattenedEntries]);

    // Calculate cumulative item counts to determine which group an item belongs to
    const cumulativeCounts = useMemo(() => {
      let sum = 0;
      return groupCounts.map((count) => {
        const result = sum;
        sum += count;
        return result;
      });
    }, [groupCounts]);

    // Determine which group a given item index belongs to
    const getGroupForItem = useCallback(
      (itemIndex: number) => {
        for (let i = cumulativeCounts.length - 1; i >= 0; i--) {
          if (itemIndex >= cumulativeCounts[i]) {
            return i;
          }
        }
        return 0;
      },
      [cumulativeCounts],
    );

    const applyAliasSuggestion = useCallback(
      async (historyEntryId: number, suggestion: AliasSuggestion) => {
        if (aliasApplyInFlightRef.current.has(historyEntryId)) {
          return;
        }
        aliasApplyInFlightRef.current.add(historyEntryId);
        setAddingAliasByEntryId((prev) => ({
          ...prev,
          [historyEntryId]: true,
        }));

        try {
          const latestDictionary =
            useSettingsStore.getState().settings?.dictionary ?? [];
          const targetEntry = latestDictionary.find(
            (entry) =>
              dictionaryEntryIdentity(entry) === suggestion.entryIdentity,
          );
          if (!targetEntry) {
            toast.error(
              t(
                "settings.history.aliasAction.entryUnavailable",
                "Dictionary changed. Please try again.",
              ),
            );
            return;
          }

          const nextAliases = normalizeAliases(
            [...(targetEntry.aliases ?? []), suggestion.alias],
            targetEntry.input,
          );

          if (nextAliases.length === (targetEntry.aliases ?? []).length) {
            toast.message(
              t(
                "settings.history.aliasAction.exists",
                "Alias already exists in dictionary.",
              ),
            );
            return;
          }

          const nextDictionary = latestDictionary.map((entry) =>
            dictionaryEntryIdentity(entry) === suggestion.entryIdentity
              ? {
                  ...entry,
                  aliases: nextAliases,
                }
              : entry,
          );

          await updateSetting("dictionary", nextDictionary);
          toast.success(
            t("settings.history.aliasAction.addedTitle", "Alias added"),
            {
              description: (
                <span>
                  {t("settings.history.aliasAction.addedPrefix", "Added")}{" "}
                  <strong>{suggestion.alias}</strong>
                  {t(
                    "settings.history.aliasAction.addedSuffix",
                    ' as an alias for "{{term}}".',
                    {
                      term: targetEntry.input,
                    },
                  )}
                </span>
              ),
              duration: 9000,
              action: onNavigate
                ? {
                    label: t(
                      "settings.history.aliasAction.openDictionary",
                      "Open Dictionary",
                    ),
                    onClick: () => onNavigate("dictionary"),
                  }
                : undefined,
            },
          );
          logInfo(
            `[HistoryAlias] Added alias="${suggestion.alias}" target="${targetEntry.input}" history_id=${historyEntryId} score=${suggestion.score.toFixed(3)}`,
            "fe-history",
          );
        } catch (error) {
          logError(`Failed to apply alias suggestion: ${error}`, "fe-history");
          toast.error(
            t(
              "settings.history.aliasAction.failed",
              "Failed to add alias. Please try again.",
            ),
          );
        } finally {
          aliasApplyInFlightRef.current.delete(historyEntryId);
          setAddingAliasByEntryId((prev) => {
            const next = { ...prev };
            delete next[historyEntryId];
            return next;
          });
        }
      },
      [onNavigate, t, updateSetting],
    );

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
      const aliasSuggestion = aliasSuggestionByHistoryId.get(entry.id);
      return (
        <div className="px-6 pb-1">
          <TimelineItem
            entry={entry}
            onToggleSaved={() => onToggleSaved(entry.id)}
            onCopyText={() => copyToClipboard(getHistoryCopyText(entry))}
            deleteAudio={onDelete}
            searchQuery={searchQuery}
            aliasSuggestion={aliasSuggestion}
            onAddAliasSuggestion={
              aliasSuggestion
                ? () => applyAliasSuggestion(entry.id, aliasSuggestion)
                : undefined
            }
            isAddingAliasSuggestion={Boolean(addingAliasByEntryId[entry.id])}
          />
        </div>
      );
    };

    const renderGroup = (index: number) => {
      const date = sortedDates[index];
      // Hide the first group header when using customScrollParent since
      // the external sticky header already shows it
      if (scrollContainer && index === 0) {
        return <div className="h-0" />;
      }
      return (
        <div className="bg-card/95 backdrop-blur supports-[backdrop-filter]:bg-card/85 py-3 mb-2 border-b border-border/60 shadow-sm px-5">
          <h3 className="text-xs font-bold text-primary/80 uppercase tracking-widest px-1">
            {date}
          </h3>
        </div>
      );
    };

    const Footer = () => {
      // Try to maintain consistent height to avoid jumps
      // py-4 (1rem top + 1rem bottom = 32px) + icon (24px) = 56px => h-14
      return (
        <div className="h-14 flex justify-center w-full items-center">
          {loading && hasMore && (
            <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
          )}
        </div>
      );
    };

    const topValue =
      typeof stickyTopOffset === "number"
        ? `${stickyTopOffset}px`
        : stickyTopOffset;
    const currentDate = sortedDates[currentGroupIndex] || sortedDates[0];

    return (
      <TooltipProvider delayDuration={300}>
        {/* External sticky header - only shown when using customScrollParent */}
        {scrollContainer && sortedDates.length > 0 && (
          <div
            className="bg-card/95 backdrop-blur supports-[backdrop-filter]:bg-card/85 py-3 border-b border-border/60 shadow-sm px-6 mb-2"
            style={{ position: "sticky", top: topValue, zIndex: 15 }}
          >
            <h3 className="text-xs font-bold text-primary/80 uppercase tracking-widest px-1">
              {currentDate}
            </h3>
          </div>
        )}
        <GroupedVirtuoso
          ref={virtuosoRef}
          style={
            scrollContainer ? { height: "auto" } : { height: "100%", flex: 1 }
          }
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
  },
);

interface TimelineItemProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => void;
  deleteAudio: (id: number) => Promise<void>;
  searchQuery?: string;
  aliasSuggestion?: AliasSuggestion;
  onAddAliasSuggestion?: () => Promise<void>;
  isAddingAliasSuggestion?: boolean;
}

const TimelineItem: React.FC<TimelineItemProps> = React.memo(
  ({
    entry,
    onToggleSaved,
    onCopyText,
    deleteAudio,
    searchQuery = "",
    aliasSuggestion,
    onAddAliasSuggestion,
    isAddingAliasSuggestion = false,
  }) => {
    const { t, i18n } = useTranslation();
    const [showCopied, setShowCopied] = useState(false);
    const [isDeleting, setIsDeleting] = useState(false);
    const [showOriginalTranscript, setShowOriginalTranscript] = useState(false);

    const primaryText = getHistoryPrimaryText(entry);
    const rawText = getHistoryRawText(entry);
    const canToggleOriginalTranscript = shouldShowOriginalTranscript(entry);
    const rawOnlyMatch = isRawOnlyHistoryMatch(entry, searchQuery);
    const originalTranscriptTooltip = showOriginalTranscript
      ? t("settings.history.hideOriginalTranscript")
      : t("settings.history.showOriginalTranscript");

    useEffect(() => {
      setShowOriginalTranscript(false);
    }, [entry.id]);

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
          },
        };
      }

      // Mac/Windows: Use direct src via convertFileSrc for instant playback
      return {
        src: convertFileSrc(entry.file_path),
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

    const handleAddAliasSuggestion = async () => {
      if (!onAddAliasSuggestion || isAddingAliasSuggestion) return;
      await onAddAliasSuggestion();
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
        ),
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
            {highlightText(primaryText, searchQuery)}
          </p>
          {rawOnlyMatch && !showOriginalTranscript && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-[11px] text-muted-foreground border border-border/60 bg-muted/30 hover:bg-muted/60"
              onClick={() => setShowOriginalTranscript(true)}
            >
              {t("settings.history.matchInOriginalTranscript")}
            </Button>
          )}
          {canToggleOriginalTranscript && showOriginalTranscript && (
            <div
              id={`history-original-${entry.id}`}
              className="rounded-md border border-border/60 bg-muted/40 px-3 py-2"
            >
              <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                {t("settings.history.originalTranscript")}
              </p>
              <p className="mt-1 text-sm leading-relaxed text-muted-foreground break-words">
                {highlightText(rawText, searchQuery)}
              </p>
            </div>
          )}
          {(audioProps.src || audioProps.onLoadRequest) && (
            <AudioPlayer {...audioProps} className="w-full" />
          )}
        </div>

        {/* Actions Column - Visible on Group Hover */}
        <div className="flex items-center gap-1 ml-2 shrink-0 self-start text-muted-foreground/40 hover:text-muted-foreground transition-colors duration-200">
          {aliasSuggestion && (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="inline-flex">
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-sky-500/80 hover:text-sky-600 hover:bg-sky-500/10"
                    onClick={() => void handleAddAliasSuggestion()}
                    disabled={isAddingAliasSuggestion}
                  >
                    {isAddingAliasSuggestion ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <Sparkles className="w-4 h-4" />
                    )}
                    <span className="sr-only">
                      {t(
                        "settings.history.aliasAction.button",
                        "Add suggested alias",
                      )}
                    </span>
                  </Button>
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <p>
                  {isAddingAliasSuggestion
                    ? t(
                        "settings.history.aliasAction.adding",
                        "Adding alias...",
                      )
                    : t(
                        "settings.history.aliasAction.tooltip",
                        'Add "{{alias}}" as an alias for "{{term}}"',
                        {
                          alias: aliasSuggestion.alias,
                          term: aliasSuggestion.entryInput,
                        },
                      )}
                </p>
              </TooltipContent>
            </Tooltip>
          )}

          {canToggleOriginalTranscript && (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="inline-flex">
                  <Button
                    variant="ghost"
                    size="icon"
                    className={`h-8 w-8 ${
                      showOriginalTranscript
                        ? "text-foreground bg-accent/60"
                        : "text-muted-foreground/70 hover:text-foreground"
                    }`}
                    title={originalTranscriptTooltip}
                    aria-label={originalTranscriptTooltip}
                    aria-expanded={showOriginalTranscript}
                    aria-controls={`history-original-${entry.id}`}
                    onClick={() =>
                      setShowOriginalTranscript((currentValue) => !currentValue)
                    }
                  >
                    <FileText className="w-4 h-4" />
                    <span className="sr-only">{originalTranscriptTooltip}</span>
                  </Button>
                </span>
              </TooltipTrigger>
              <TooltipContent side="top" sideOffset={8} collisionPadding={8}>
                <p>
                  {originalTranscriptTooltip}
                </p>
              </TooltipContent>
            </Tooltip>
          )}

          <Tooltip>
            <TooltipTrigger asChild>
              <span className="inline-flex">
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
              </span>
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
              <span className="inline-flex">
                <Button
                  variant="ghost"
                  size="icon"
                  className={`h-8 w-8 ${
                    entry.saved ? "text-yellow-500 hover:text-yellow-600" : ""
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
              </span>
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
              <span className="inline-flex">
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
              </span>
            </TooltipTrigger>
            <TooltipContent>
              <p>{t("settings.history.delete")}</p>
            </TooltipContent>
          </Tooltip>
        </div>
      </div>
    );
  },
);
