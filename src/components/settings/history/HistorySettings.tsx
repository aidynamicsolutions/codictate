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
import { useHistory } from "@/hooks/useHistory";
import { HistoryList } from "@/components/shared/HistoryList";

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
      <Card className="bg-card/50 border-border/50 backdrop-blur supports-[backdrop-filter]:bg-card/50">
        <CardContent className="p-4 flex flex-col gap-4">
          <div className="flex items-center justify-between">
            <div>
              <h4 className="text-sm font-medium">
                {t("settings.history.storageTitle", "History storage usage")}
              </h4>
              <p className="text-xs text-muted-foreground mt-1">
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
            <div className="flex gap-3 items-center">
              <Select
                disabled={pruning}
                onValueChange={(val) => setPruneDays(parseInt(val))}
                value="" // Always reset to empty to allow re-selection
              >
                <SelectTrigger className="w-[150px] h-8 text-xs">
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
                    {
                      days: 30,
                      label: t("settings.history.pruneOptions.30d"),
                    },
                    {
                      days: 90,
                      label: t("settings.history.pruneOptions.3m"),
                    },
                    {
                      days: 365,
                      label: t("settings.history.pruneOptions.1y"),
                    },
                  ].map(({ days, label }) => (
                    <SelectItem
                      key={days}
                      value={days.toString()}
                      className="text-xs"
                    >
                      {t("settings.history.pruneLabel", {
                        label,
                      })}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              <Button
                onClick={onClearAll}
                disabled={!hasHistory}
                variant="outline"
                size="sm"
                className="flex items-center gap-2 h-8 text-xs font-medium text-destructive hover:bg-destructive/10 hover:text-destructive border-destructive/30"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>{t("settings.history.clearAll")}</span>
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

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
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setPruneDays(null)}
              disabled={pruning}
            >
              {t("common.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={handlePruneConfirm}
              disabled={pruning}
            >
              {pruning ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
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
    <div className="max-w-3xl w-full mx-auto space-y-6 flex flex-col flex-1 min-h-0">
      <HistoryStorage
        onPrune={loadHistoryEntries}
        onClearAll={() => setShowClearDialog(true)}
        hasHistory={historyEntries.length > 0}
      />
      <Card className="w-full flex-1 min-h-0 flex flex-col animate-in fade-in slide-in-from-bottom-2 duration-500 bg-card border-border shadow-sm">
        <CardHeader className="pb-3 flex flex-row items-center justify-between space-y-0">
          <CardTitle className="text-sm font-semibold uppercase tracking-wide text-primary font-heading">
            {t("settings.history.title")}
          </CardTitle>
          <div className="flex items-center gap-2">
            <OpenRecordingsButton
              onClick={openRecordingsFolder}
              label={t("settings.history.openFolder")}
            />
          </div>
        </CardHeader>
        <CardContent className="p-0 flex-1 min-h-0 flex flex-col">
          <div className="p-4 border-b border-border/50 bg-muted/20">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <Input
                id="history-search-input"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={t("settings.history.searchPlaceholder")}
                className="pl-9 pr-24 h-9 bg-background/50"
              />
              <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none border border-border rounded px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground bg-muted/50">
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
            />
          </div>
        </CardContent>
      </Card>

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
              onClick={handleClearAllHistory}
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
