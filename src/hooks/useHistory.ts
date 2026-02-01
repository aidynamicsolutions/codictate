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
  const [loading, setLoading] = useState(true);
  const [isClearing, setIsClearing] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const debouncedSearchQuery = useDebounce(searchQuery, 300);

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
    } catch (error) {
      logError(`Failed to clear all history: ${error}`, "fe-history");
      throw error;
    } finally {
      setIsClearing(false);
    }
  };

  const filteredEntries = useMemo(() => {
    if (!debouncedSearchQuery.trim()) {
      return historyEntries;
    }
    const lowerQuery = debouncedSearchQuery.toLowerCase();
    return historyEntries.filter((entry) =>
      entry.transcription_text.toLowerCase().includes(lowerQuery)
    );
  }, [historyEntries, debouncedSearchQuery]);

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
    loadHistoryEntries,
    toggleSaved,
    deleteAudioEntry,
    clearAllHistory,
    getAudioUrl,
    isClearing,
    searchQuery,
    setSearchQuery,
    filteredEntries,
    debouncedSearchQuery,
  };
};
