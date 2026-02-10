import React, { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/shared/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/shared/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/shared/ui/select";
import { Skeleton } from "@/components/shared/ui/skeleton";
import { Input } from "@/components/shared/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/shared/ui/dialog";
import { Trash2, FolderOpen, Loader2, Search } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { commands, type HistoryStats } from "@/bindings";
import { logError } from "@/utils/logging";
import { useHistory, getFilterEmptyState } from "@/hooks/useHistory";
import { HistoryList } from "@/components/shared/HistoryList";
import { HistoryFilterDropdown } from "@/components/shared/HistoryFilterDropdown";



const HistoryStorage: React.FC<{
  onPrune: () => void;
  onClearAll: () => void;
  hasHistory: boolean;
}> = ({ onPrune, onClearAll, hasHistory }) => {
  const { t } = useTranslation();
  const [stats, setStats] = useState<HistoryStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [pruning, setPruning] = useState(false);
  const [pruneDays, setPruneDays] = useState<number | null>(null);

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      const result = await commands.getHistoryStorageUsage();
      if (result.status === "ok") {
        setStats(result.data);
      }
    } catch (error) {
      logError(`Failed to load history stats: ${error}`, "fe-history");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStats();

    const unlistenPromise = listen("history-updated", () => {
      loadStats();
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [loadStats]);

  const handlePruneConfirm = async () => {
    if (!pruneDays) return;

    setPruning(true);
    try {
      const result = await commands.pruneHistory(pruneDays);
      if (result.status === "ok") {
        const count = result.data;
        await loadStats();
        onPrune();

        toast.success(
          t("settings.history.pruneSuccessTitle", "History Pruned"),
          {
            description: t("settings.history.pruneSuccessDescription", {
              count,
              days: pruneDays,
            }),
          }
        );
      }
    } catch (error) {
      logError(`Failed to prune history: ${error}`, "fe-history");
      toast.error(t("common.error"), {
        description: t(
          "settings.history.pruneError",
          "Failed to prune history"
        ),
      });
    } finally {
      setPruning(false);
      setPruneDays(null);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  if (loading && !stats) return <Skeleton className="h-20 w-full" />;

  return (
    <>
      <div className="w-full">
         <div className="px-1 mb-3">
            <h3 className="text-sm font-medium text-muted-foreground tracking-wide pl-1">
                {t("settings.history.managementTitle", "Storage & Management")}
            </h3>
         </div>
        <Card className="bg-card border-border shadow-sm rounded-xl overflow-hidden">
            <CardContent className="p-0">
            <div className="flex items-center justify-between p-4">
                <div className="flex flex-col gap-1.5 mr-6">
                    <div className="flex items-center gap-2">
                        <h4 className="text-sm font-medium text-foreground/90">
                            {t("settings.history.storageTitle", "History Storage")}
                        </h4>
                        <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => commands.openRecordingsFolder()}
                            className="h-6 w-6 text-muted-foreground hover:text-foreground"
                            title={t("settings.history.openFolder")}
                        >
                            <FolderOpen className="w-4 h-4" />
                        </Button>
                    </div>
                    <p className="text-[13px] text-muted-foreground/80 leading-relaxed font-normal">
                        {stats ? (
                        t("settings.history.storageDescription", {
                            size: formatBytes(stats.total_size_bytes),
                            count: stats.total_entries,
                        })
                        ) : (
                        "..."
                        )}
                    </p>
                </div>
                <div className="flex gap-3 items-center shrink-0">
                    <Select
                        disabled={pruning}
                        onValueChange={(val) => setPruneDays(parseInt(val))}
                        value="" 
                    >
                        <SelectTrigger className="w-[180px] h-9 text-sm font-medium shadow-sm bg-secondary border-none text-secondary-foreground">
                            <SelectValue
                                placeholder={t(
                                "settings.history.prunePlaceholder",
                                "Prune history..."
                                )}
                            />
                        </SelectTrigger>
                        <SelectContent position="popper">
                        {[
                            { days: 3, label: t("settings.history.pruneOptions.3d") },
                            { days: 7, label: t("settings.history.pruneOptions.7d") },
                            { days: 30, label: t("settings.history.pruneOptions.30d") },
                            { days: 90, label: t("settings.history.pruneOptions.3m") },
                            { days: 365, label: t("settings.history.pruneOptions.1y") },
                        ].map(({ days, label }) => (
                            <SelectItem key={days} value={days.toString()} className="text-xs">
                            {t("settings.history.pruneLabel", { label })}
                            </SelectItem>
                        ))}
                        </SelectContent>
                    </Select>

                    <Button
                        onClick={onClearAll}
                        disabled={!hasHistory}
                        variant="destructive"
                        size="sm"
                        className="h-9 px-4 min-w-[6rem] text-sm font-medium shadow-sm rounded-md bg-destructive hover:bg-destructive/90 text-destructive-foreground"
                    >
                        <span>{t("settings.history.clearAll", "Clear All")}</span>
                    </Button>
                </div>
            </div>
            </CardContent>
        </Card>
      </div>

      {/* Prune Confirmation Dialog */}
      <Dialog
        open={pruneDays !== null}
        onOpenChange={(open) => !open && setPruneDays(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("settings.history.pruneConfirmTitle", "Prune History")}
            </DialogTitle>
            <DialogDescription>
              {t("settings.history.pruneConfirmDescription", {
                days: pruneDays,
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="gap-3 sm:space-x-0">
            <Button
              variant="outline"
              onClick={() => setPruneDays(null)}
              disabled={pruning}
              className="h-9 px-4 min-w-[5rem] rounded-md text-sm font-medium"
            >
              {t("common.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={handlePruneConfirm}
              disabled={pruning}
              className="h-9 px-4 min-w-[5rem] rounded-md shadow-sm text-sm font-medium"
            >
              {pruning ? (
                <>
                  <Loader2 className="w-3.5 h-3.5 mr-2 animate-spin" />
                  {t("common.pruning", "Pruning...")}
                </>
              ) : (
                t("common.confirm", "Confirm")
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

export const HistorySettings: React.FC = () => {
  const { t } = useTranslation();
  const [showClearDialog, setShowClearDialog] = useState(false);

  const {
    historyEntries,
    loading,
    groupedEntries,
    sortedDates,
    loadHistoryEntries,
    loadMore,
    hasMore,
    toggleSaved,
    deleteAudioEntry,
    clearAllHistory,
    getAudioUrl,
    isClearing,
    searchQuery,
    setSearchQuery,
    filteredEntries,
    debouncedSearchQuery,
    filter,
    setFilter,
    hasActiveFilters,
    clearFilters,
  } = useHistory();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "f") {
        e.preventDefault();
        const input = document.getElementById("history-search-input");
        if (input) {
          input.focus();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleClearAllHistory = async () => {
    await clearAllHistory();
    setShowClearDialog(false);
  };

  const openRecordingsFolder = async () => {
    try {
      await commands.openRecordingsFolder();
    } catch (error) {
      logError(`Failed to open recordings folder: ${error}`, "fe-history");
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-8 flex flex-col flex-1 min-h-0 pb-0 -mb-8 relative py-4">
      <HistoryStorage
        onPrune={loadHistoryEntries}
        onClearAll={() => setShowClearDialog(true)}
        hasHistory={historyEntries.length > 0}
      />
      
      <div className="flex-1 min-h-0 flex flex-col space-y-3">
         <div className="px-1">
            <h3 className="text-sm font-medium text-muted-foreground tracking-wide pl-1">
                {t("settings.history.title")}
            </h3>
         </div>
         
         <Card className="w-full flex-1 min-h-0 flex flex-col bg-card/50 backdrop-blur-sm ring-0 border-x border-t border-border shadow-sm rounded-t-xl rounded-b-none overflow-hidden">
            <CardContent className="p-0 flex-1 min-h-0 flex flex-col">
            <div className="p-4 border-b border-border/40 bg-muted/10 pb-3">
                <div className="flex items-center gap-2">
                <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground/70 z-10" />
                <Input
                    id="history-search-input"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder={t("settings.history.searchPlaceholder")}
                    className="pl-9 pr-16 h-9 bg-background/50 border-border/60 focus:bg-background transition-colors w-full"
                />
                <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none border border-border/60 rounded px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground bg-muted/30">
                    {searchQuery ? (
                    <span>
                        {t(
                        filteredEntries.length === 1
                            ? "settings.history.foundResult"
                            : "settings.history.foundResults",
                        {
                            count: filteredEntries.length,
                        }
                        )}
                    </span>
                    ) : (
                    <>
                        {/* eslint-disable-next-line i18next/no-literal-string */}
                        <span className="text-xs">⌘</span>F
                    </>
                    )}
                </div>
                </div>
                <HistoryFilterDropdown
                  filter={filter}
                  onFilterChange={setFilter}
                  hasActiveFilters={hasActiveFilters}
                  onClearFilters={clearFilters}
                />
                </div>
            </div>
            <div className="flex-1 min-h-0 flex flex-col">
                <HistoryList
                loading={loading}
                historyEntries={historyEntries}
                sortedDates={sortedDates}
                groupedEntries={groupedEntries}
                onToggleSaved={toggleSaved}
                onDelete={deleteAudioEntry}
                searchQuery={debouncedSearchQuery}
                loadMore={loadMore}
                hasMore={hasMore}
                emptyMessage={
                  getFilterEmptyState(filter, hasActiveFilters, t).emptyMessage
                }
                emptyDescription={
                  getFilterEmptyState(filter, hasActiveFilters, t).emptyDescription
                }
                />
            </div>
            </CardContent>
        </Card>
        <div className="absolute bottom-0 left-0 right-0 h-10 bg-linear-to-t from-background via-background/60 to-transparent pointer-events-none z-10 backdrop-blur-[1px]" />
      </div>

      {/* Clear All Confirmation Dialog */}
      <Dialog open={showClearDialog} onOpenChange={setShowClearDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("settings.history.clearAllConfirmTitle")}
            </DialogTitle>
            <DialogDescription>
              {t("settings.history.clearAllConfirmDescription", {
                count: historyEntries.length,
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="gap-3 sm:space-x-0">
            <Button
              variant="outline"
              onClick={() => setShowClearDialog(false)}
              disabled={isClearing}
              className="h-9 px-4 min-w-[5rem] rounded-md text-sm font-medium"
            >
              {t("common.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={handleClearAllHistory}
              disabled={isClearing}
              className="h-9 px-4 min-w-[5rem] rounded-md shadow-sm text-sm font-medium"
            >
              {isClearing ? (
                <>
                  <Loader2 className="w-3.5 h-3.5 mr-2 animate-spin" />
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
