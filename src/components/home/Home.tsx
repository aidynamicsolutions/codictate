import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { logInfo, logError } from "@/utils/logging";
import { listen } from "@tauri-apps/api/event";
import { GettingStarted } from "./GettingStarted";
import { WhatsNew } from "./WhatsNew";
import { StatsOverview } from "./StatsOverview";

interface Stats {
  total_words: number;
  total_duration_minutes: number;
  wpm: number;
  time_saved_minutes: number;
  streak_days: number;
  faster_than_typing_percentage: number;
}

export default function Home({
  onNavigate,
}: {
  onNavigate: (section: string) => void;
}) {
  const { t } = useTranslation();
  const [username, setUsername] = useState("User");
  const [stats, setStats] = useState<Stats | null>(null);

  useEffect(() => {
    logInfo("[Home] Component mounted, loading initial data...", "fe-home");
    loadData();

    // Listen for history updates to refresh stats
    const unlistenHistory = listen("history-updated", () => {
      logInfo(
        "[Home] Received history-updated event, reloading data...",
        "fe-home"
      );
      loadData();
    });

    // Refresh stats when window gains focus
    const unlistenFocus = listen("tauri://focus", () => {
      logInfo("[Home] Window focused, refreshing data...", "fe-home");
      loadData();
    });

    return () => {
      unlistenHistory.then((unlisten) => unlisten());
      unlistenFocus.then((unlisten) => unlisten());
    };
  }, []);

  const loadData = async () => {
    try {
      logInfo("[Home] Fetching user profile...", "fe-home");
      const profileResult = await commands.getUserProfileCommand();
      logInfo(
        `[Home] Profile result: ${JSON.stringify(profileResult)}`,
        "fe-home"
      );
      if (profileResult.status === "ok" && profileResult.data.user_name) {
        setUsername(profileResult.data.user_name);
      }

      logInfo("[Home] Fetching home stats...", "fe-home");
      const homeStats = await commands.getHomeStats();
      logInfo(
        `[Home] Raw stats from backend: total_words=${homeStats.status === "ok" ? homeStats.data.total_words : "error"}, wpm=${homeStats.status === "ok" ? homeStats.data.wpm : "error"}`,
        "fe-home"
      );

      if (homeStats && homeStats.status === "ok") {
        const statsData = homeStats.data;
        const newStats = {
          total_words: Number(statsData.total_words),
          total_duration_minutes: statsData.total_duration_minutes,
          wpm: statsData.wpm,
          time_saved_minutes: statsData.time_saved_minutes,
          streak_days: Number(statsData.streak_days),
          faster_than_typing_percentage: statsData.faster_than_typing_percentage,
        };
        logInfo(
          `[Home] Setting stats: total_words=${newStats.total_words} (prev=${stats?.total_words ?? "null"})`,
          "fe-home"
        );
        setStats(newStats);
      }
    } catch (e) {
      logError(`[Home] Failed to load home data: ${e}`, "fe-home");
    }
  };

  return (
    <div className="flex flex-col gap-8 p-8 max-w-5xl mx-auto w-full pb-20 animate-in fade-in slide-in-from-bottom-2 duration-500">
      <div className="flex flex-col gap-2">
        <h1 className="text-4xl font-bold tracking-tight">
          {t("home.welcome")} {username}
        </h1>
      </div>

      <StatsOverview stats={stats} />

      <WhatsNew />
      <GettingStarted onNavigate={onNavigate} />
    </div>
  );
}
