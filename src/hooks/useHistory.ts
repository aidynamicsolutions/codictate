import { useState, useEffect, useCallback, useMemo } from "react";
import { listen } from "@tauri-apps/api/event";
import { platform } from "@tauri-apps/plugin-os";
import { readFile } from "@tauri-apps/plugin-fs";
import { convertFileSrc } from "@tauri-apps/api/core";
import { commands, type HistoryEntry } from "@/bindings";
import { logError, logInfo } from "@/utils/logging";
import { useTranslation } from "react-i18next";
import { formatDate } from "@/utils/dateFormat";

const IS_LINUX = platform() === "linux";

export type HistoryFilter = "all" | "starred" | "today" | "this_week" | "this_month" | "this_year";

/** Compute a unix-seconds cutoff timestamp for the given time period in local timezone */
function computeTimePeriodStart(filter: HistoryFilter): number | null {
  if (filter === "all" || filter === "starred") return null;
  const now = new Date();
  let cutoff: Date;
  switch (filter) {
    case "today":
      cutoff = new Date(now.getFullYear(), now.getMonth(), now.getDate());
      break;
    case "this_week": {
      const day = now.getDay(); // 0=Sun
      const diff = day === 0 ? 6 : day - 1; // Monday start (ISO 8601)
      cutoff = new Date(now.getFullYear(), now.getMonth(), now.getDate() - diff);
      break;
    }
    case "this_month":
      cutoff = new Date(now.getFullYear(), now.getMonth(), 1);
      break;
    case "this_year":
      cutoff = new Date(now.getFullYear(), 0, 1);
      break;
  }
  return Math.floor(cutoff.getTime() / 1000);
}

import { useDebounce } from "./useDebounce";

/** Returns filter-aware empty-state i18n keys (or undefined for default behavior) */
export function getFilterEmptyState(
  filter: HistoryFilter,
  hasActiveFilters: boolean,
  t: (key: string) => string
): { emptyMessage?: string; emptyDescription?: string } {
  if (!hasActiveFilters) return {};
  if (filter === "starred") {
    return {
      emptyMessage: t("settings.history.filter.noStarred"),
      emptyDescription: t("settings.history.filter.noStarredDescription"),
    };
  }
  return {
    emptyMessage: t("settings.history.filter.noPeriod"),
    emptyDescription: t("settings.history.filter.noPeriodDescription"),
  };
}

export interface UseHistoryReturn {
  historyEntries: HistoryEntry[];
  loading: boolean;
  groupedEntries: { [key: string]: HistoryEntry[] };
  sortedDates: string[];
  loadHistoryEntries: () => Promise<void>;
  loadMore: () => Promise<void>;
  hasMore: boolean;
  toggleSaved: (id: number) => Promise<void>;
  deleteAudioEntry: (id: number) => Promise<void>;
  clearAllHistory: () => Promise<void>;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  isClearing: boolean;
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  filteredEntries: HistoryEntry[];
  debouncedSearchQuery: string;
  filter: HistoryFilter;
  setFilter: (value: HistoryFilter) => void;
  hasActiveFilters: boolean;
  clearFilters: () => void;
}

export const useHistory = (): UseHistoryReturn => {
  const { i18n } = useTranslation();
  const [historyEntries, setHistoryEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(false); // Start false, logic handles it
  const [hasMore, setHasMore] = useState(true);
  const [offset, setOffset] = useState(0);
  const [isClearing, setIsClearing] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const debouncedSearchQuery = useDebounce(searchQuery, 300);
  const [filter, setFilter] = useState<HistoryFilter>("all");


  const hasActiveFilters = filter !== "all";
  const clearFilters = useCallback(() => {
    setFilter("all");
  }, []);

  const LIMIT = 50;

  // Reset offset and entries when search query or filters change
  useEffect(() => {
    setOffset(0);
    setHasMore(true);
  }, [debouncedSearchQuery, filter]);

  const loadHistoryEntries = useCallback(async (isLoadMore = false) => {
    // If we receive a request to load more but we know there's no more, stop.
    // However, we must allow the first load (offset 0) even if hasMore might seem false (it resets on search)
    if (isLoadMore && !hasMore) return;

    setLoading(true);
    try {
      // Use current offset if loading more, otherwise 0
      const currentOffset = isLoadMore ? offset : 0;
      
      const timePeriodStart = computeTimePeriodStart(filter);
      const isStarred = filter === "starred";
      const result = await commands.getHistoryEntries(LIMIT, currentOffset, debouncedSearchQuery || null, isStarred, timePeriodStart);
      
      if (result.status === "ok") {
        const newEntries = result.data;
        
        if (isLoadMore) {
          setHistoryEntries(prev => [...prev, ...newEntries]);
        } else {
          setHistoryEntries(newEntries);
        }

        // Prepare offset for next load
        if (newEntries.length < LIMIT) {
          setHasMore(false);
        } else {
          setHasMore(true);
          setOffset(currentOffset + LIMIT);
        }
      }
    } catch (error) {
      logError(`Failed to load history entries: ${error}`, "fe-history");
    } finally {
      setLoading(false);
    }
  }, [offset, hasMore, debouncedSearchQuery, filter]);

  // Initial load and search reaction
  // We use a ref to track if it's the very first mount vs search update
  useEffect(() => {
    loadHistoryEntries(false);
  }, [debouncedSearchQuery, filter]); 

  // Reload on updates (e.g. deletion)
  // Logic: if an item is deleted, re-fetching whole list might be expensive.
  // Ideally, we just remove it locally.
  // For now, we'll keep the simple "reload from start" logic but it might be jarring for deep scroll.
  // TODO: Optimistically update local state instead of full reload for deletions.

  useEffect(() => {
    const setupListener = async () => {
      const unlisten = await listen("history-updated", () => {
        logInfo("History updated, reloading entries...", "fe-history");
        // For simplicity, reload from scratch to ensure consistency
        setOffset(0);
        setHasMore(true);
        loadHistoryEntries(false);
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

  const toggleSaved = useCallback(async (id: number) => {
    try {
      await commands.toggleHistoryEntrySaved(id);
      const isStarredFilter = filter === "starred";
      // Optimistic update — if "starred only" filter is active and user
      // unstars an entry, remove it from the list since it no longer matches
      setHistoryEntries(prev => {
        const entry = prev.find(e => e.id === id);
        if (isStarredFilter && entry?.saved) {
          // Entry is being unstarred while starred filter is active — remove it
          return prev.filter(e => e.id !== id);
        }
        return prev.map(e => e.id === id ? { ...e, saved: !e.saved } : e);
      });
    } catch (error) {
      logError(`Failed to toggle saved status: ${error}`, "fe-history");
    }
  }, [filter]);

  const getAudioUrl = useCallback(async (fileName: string) => {
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
  }, []);

  const deleteAudioEntry = async (id: number) => {
    try {
      await commands.deleteHistoryEntry(id);
      // Optimistic update
      setHistoryEntries(prev => prev.filter(e => e.id !== id));
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
      setHistoryEntries([]);
      setOffset(0);
      setHasMore(false);
    } catch (error) {
      logError(`Failed to clear all history: ${error}`, "fe-history");
      throw error;
    } finally {
      setIsClearing(false);
    }
  };

  // We no longer filter client side
  const filteredEntries = historyEntries;

  const groupedEntries = useMemo(() => {
    const groups: { [key: string]: HistoryEntry[] } = {};
    filteredEntries.forEach((entry) => {
      // Create a localized date string for grouping
      const dateKey = formatDate(String(entry.timestamp), i18n.language);
      if (!groups[dateKey]) {
        groups[dateKey] = [];
      }
      groups[dateKey].push(entry);
    });
    return groups;
  }, [filteredEntries, i18n.language]);

  // Sort dates descending (newest first)
  const sortedDates = useMemo(() => {
    // Original order from DB should already be sorted descending generally,
    // but preserving strict order of keys here
    return Object.keys(groupedEntries).sort((a, b) => {
      // We can pick the first entry of each group to compare timestamps since they are grouped by date
      const timestampA = groupedEntries[a][0].timestamp;
      const timestampB = groupedEntries[b][0].timestamp;
      return timestampB - timestampA;
    });
  }, [groupedEntries]);

  return {
    historyEntries,
    loading,
    groupedEntries,
    sortedDates,
    loadHistoryEntries: () => loadHistoryEntries(false), // Reset-load
    loadMore: () => loadHistoryEntries(true), // Append-load
    hasMore,
    toggleSaved,
    deleteAudioEntry,
    clearAllHistory,
    getAudioUrl,
    isClearing,
    searchQuery,
    setSearchQuery,
    filteredEntries, // Kept for interface compatibility but same as historyEntries
    debouncedSearchQuery,
    filter,
    setFilter,
    hasActiveFilters,
    clearFilters,
  };
};
