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

import { useDebounce } from "./useDebounce";

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

  const LIMIT = 50;

  // Reset offset and entries when search query changes
  useEffect(() => {
    setOffset(0);
    setHasMore(true);
    // We intentionally don't clear entries here to avoid flicker, the next load will replace them
    // But for correct UX we should probably trigger a load immediately or rely on the effect below
  }, [debouncedSearchQuery]);

  const loadHistoryEntries = useCallback(async (isLoadMore = false) => {
    // If we receive a request to load more but we know there's no more, stop.
    // However, we must allow the first load (offset 0) even if hasMore might seem false (it resets on search)
    if (isLoadMore && !hasMore) return;

    setLoading(true);
    try {
      // Use current offset if loading more, otherwise 0
      const currentOffset = isLoadMore ? offset : 0;
      
      const result = await commands.getHistoryEntries(LIMIT, currentOffset, debouncedSearchQuery || null);
      
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
  }, [offset, hasMore, debouncedSearchQuery]);

  // Initial load and search reaction
  // We use a ref to track if it's the very first mount vs search update
  useEffect(() => {
    loadHistoryEntries(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debouncedSearchQuery]); 

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

  const toggleSaved = async (id: number) => {
    try {
      await commands.toggleHistoryEntrySaved(id);
      // Optimistic update
      setHistoryEntries(prev => prev.map(e => e.id === id ? { ...e, saved: !e.saved } : e));
    } catch (error) {
      logError(`Failed to toggle saved status: ${error}`, "fe-history");
    }
  };

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
  };
};
