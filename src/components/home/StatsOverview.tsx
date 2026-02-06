import React from "react";
import { useTranslation } from "react-i18next";
import { Card, CardContent } from "@/components/shared/ui/card";
import { Zap, Clock, Type, Trophy, Info } from "lucide-react";
import { AnimatedCounter } from "@/components/shared/AnimatedCounter";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";

interface Stats {
  total_words: number;
  total_duration_minutes: number;
  wpm: number;
  time_saved_minutes: number;
  streak_days: number;
  faster_than_typing_percentage: number;
}

interface StatsOverviewProps {
  stats: Stats | null;
}

const StatCard = ({
  icon: Icon,
  label,
  value,
  subtext,
  highlight = false,
  tooltipText,
}: {
  icon: any;
  label: string;
  value: React.ReactNode;
  subtext?: string;
  highlight?: boolean;
  tooltipText?: string;
}) => (
  <Card
    className={`border-none shadow-sm ${
      highlight ? "bg-primary/5" : "bg-card"
    }`}
  >
    <CardContent className="p-6 flex flex-col gap-2">
      <div className="flex items-center gap-2 text-muted-foreground text-sm font-medium uppercase tracking-wider">
        <Icon size={16} />
        {label}
        {tooltipText && (
          <TooltipProvider>
            <Tooltip delayDuration={300}>
              <TooltipTrigger asChild>
                <div className="cursor-help transition-opacity hover:opacity-80">
                  <Info size={14} className="text-muted-foreground/70" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-[250px] text-xs">
                {tooltipText}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>
      <div className="text-3xl font-bold">{value}</div>
      {subtext && (
        <div className="text-sm text-muted-foreground">{subtext}</div>
      )}
    </CardContent>
  </Card>
);

export function StatsOverview({ stats }: StatsOverviewProps) {
  const { t } = useTranslation();

  const streakDays = stats?.streak_days ?? 0;
  const totalWords = stats?.total_words ?? 0;
  const timeSavedMins = stats?.time_saved_minutes ?? 0;
  const wpm = stats?.wpm ?? 0;
  const fasterThanTyping = stats?.faster_than_typing_percentage ?? 0;

  // Get streak encouragement message based on streak days
  const getStreakEncouragement = (streakDays: number): string => {
    if (streakDays === 0) return t("home.stats.streakEncouragement.zero");
    if (streakDays <= 3) return t("home.stats.streakEncouragement.low");
    if (streakDays <= 7) return t("home.stats.streakEncouragement.medium");
    return t("home.stats.streakEncouragement.high");
  };

  // Get word equivalent based on total words
  const getWordEquivalent = (totalWords: number): string => {
    const TWEET_WORDS = 50;
    const BLOG_WORDS = 800;
    const ARTICLE_WORDS = 2000;
    const SHORT_STORY_WORDS = 7500;
    const NOVELLA_WORDS = 30000;
    const NOVEL_WORDS = 80000;

    if (totalWords >= NOVEL_WORDS) {
      const count = Math.floor(totalWords / NOVEL_WORDS);
      return t("home.stats.wordEquivalent.novels", { count });
    }
    if (totalWords >= NOVELLA_WORDS) {
      const count = Math.floor(totalWords / NOVELLA_WORDS);
      return t("home.stats.wordEquivalent.novellas", { count });
    }
    if (totalWords >= SHORT_STORY_WORDS) {
      const count = Math.floor(totalWords / SHORT_STORY_WORDS);
      return t("home.stats.wordEquivalent.shortStories", { count });
    }
    if (totalWords >= ARTICLE_WORDS) {
      const count = Math.floor(totalWords / ARTICLE_WORDS);
      return t("home.stats.wordEquivalent.articles", { count });
    }
    if (totalWords >= BLOG_WORDS) {
      const count = Math.floor(totalWords / BLOG_WORDS);
      return t("home.stats.wordEquivalent.blogPosts", { count });
    }
    if (totalWords >= TWEET_WORDS) {
      const count = Math.floor(totalWords / TWEET_WORDS);
      return t("home.stats.wordEquivalent.tweets", { count });
    }
    if (totalWords >= 25) {
      return t("home.stats.wordEquivalent.almostTweet");
    }
    if (totalWords >= 1) {
      return t("home.stats.wordEquivalent.gettingStarted");
    }
    return t("home.stats.wordEquivalent.zero");
  };

  // Get time equivalent based on minutes saved
  const getTimeEquivalent = (minutes: number): string => {
    const COFFEE_BREAK_MINS = 10;
    const SHOWER_MINS = 8;
    const TV_EPISODE_MINS = 22;
    const COMMUTE_MINS = 25;
    const MOVIE_MINS = 120;
    const WORKDAY_MINS = 480;

    if (minutes >= WORKDAY_MINS) {
      const count = Math.floor(minutes / WORKDAY_MINS);
      return t("home.stats.timeEquivalent.workdays", { count });
    }
    if (minutes >= MOVIE_MINS) {
      const count = Math.floor(minutes / MOVIE_MINS);
      return t("home.stats.timeEquivalent.movies", { count });
    }
    if (minutes >= COMMUTE_MINS) {
      const count = Math.floor(minutes / COMMUTE_MINS);
      return t("home.stats.timeEquivalent.commutes", { count });
    }
    if (minutes >= TV_EPISODE_MINS) {
      const count = Math.floor(minutes / TV_EPISODE_MINS);
      return t("home.stats.timeEquivalent.tvEpisodes", { count });
    }
    if (minutes >= COFFEE_BREAK_MINS) {
      const count = Math.floor(minutes / COFFEE_BREAK_MINS);
      return t("home.stats.timeEquivalent.coffeeBreaks", { count });
    }
    if (minutes >= SHOWER_MINS) {
      const count = Math.floor(minutes / SHOWER_MINS);
      return t("home.stats.timeEquivalent.showers", { count });
    }
    if (minutes >= 5) {
      return t("home.stats.timeEquivalent.almostThere");
    }
    if (minutes >= 1) {
      return t("home.stats.timeEquivalent.gettingStarted");
    }
    if (minutes > 0) {
      return t("home.stats.timeEquivalent.everySecond");
    }
    return t("home.stats.timeEquivalent.zero");
  };

  // Format time saved in human-readable format
  const formatTimeSaved = (value: number): React.ReactNode => {
    const totalSeconds = Math.round(value * 60);
    
    // Less than 1 minute: Show seconds (Standard animation 2s)
    if (totalSeconds < 60) {
      return (
        <span className="flex items-center gap-1">
          <AnimatedCounter
            value={totalSeconds}
            duration={2000}
            formatter={(val) => Math.round(val).toString()}
          />
          <span>{totalSeconds <= 1 ? "sec" : "secs"}</span>
          <span>{totalSeconds <= 1 ? "sec" : "secs"}</span>
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="text-lg">⏱️</span>
        </span>
      );
    }

    // Less than 1 hour: Show compact mins/secs (Standard animation 2s)
    if (totalSeconds < 3600) {
      const mins = Math.floor(totalSeconds / 60);
      const secs = totalSeconds % 60;
      return (
        <span className="flex items-center gap-1">
          <span>
            <AnimatedCounter value={mins} duration={2000} />
            {"m "}
            <AnimatedCounter value={secs} duration={2000} />
            {"s"}
          </span>
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="text-lg">⏱️</span>
        </span>
      );
    }

    const totalMins = Math.floor(value);
    const MINS_PER_HOUR = 60;
    const MINS_PER_DAY = 1440; // 24 * 60

    // More than 24 hours: Show days/hours/mins (Slow aesthetic animation 3s)
    if (totalMins >= MINS_PER_DAY) {
      const days = Math.floor(totalMins / MINS_PER_DAY);
      const hours = Math.floor((totalMins % MINS_PER_DAY) / MINS_PER_HOUR);
      const mins = totalMins % MINS_PER_HOUR;
      return (
        <>
          <AnimatedCounter value={days} duration={3000} />
          {"d "}
          <AnimatedCounter value={hours} duration={3000} />
          {"h "}
          <AnimatedCounter value={mins} duration={3000} />
          {"m"}
        </>
      );
    }
    
    // More than 1 hour: Show hours/mins (Slow aesthetic animation 3s)
    if (totalMins >= MINS_PER_HOUR) {
      const hours = Math.floor(totalMins / MINS_PER_HOUR);
      const mins = totalMins % MINS_PER_HOUR;
      return (
        <>
          <AnimatedCounter value={hours} duration={3000} />
          {"h "}
          <AnimatedCounter value={mins} duration={3000} />
          {"m "}
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="text-lg">⏱️</span>
        </>
      );
    }
    
    // Fallback using standard animation
    return (
      <>
        <AnimatedCounter value={totalMins} duration={2000} />{" "}
        {totalMins <= 1 ? "min" : "mins"} 
        {/* eslint-disable-next-line i18next/no-literal-string */}
        <span className="text-lg">⏱️</span>
      </>
    );
  };

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <StatCard
        icon={Zap}
        label={t("home.stats.streak")}
        value={
          <>
            {streakDays}{" "}
            {streakDays === 1 ? t("home.stats.day") : t("home.stats.days")}{" "}
            <span className="text-lg">🔥</span>
          </>
        }
        subtext={getStreakEncouragement(streakDays)}
        tooltipText={t("home.stats.tooltips.streak")}
        highlight
      />
      <StatCard
        icon={Trophy}
        label={t("home.stats.wpm")}
        value={
          <>
            {Math.round(wpm)} <span className="text-lg">🏆</span>
          </>
        }
        subtext={
          wpm > 0 && fasterThanTyping > 0
            ? t("home.stats.fasterThanTyping", {
                val: Math.round(fasterThanTyping),
              })
            : wpm === 0
            ? t("home.stats.wpmEncouragement.zero")
            : undefined
        }
        tooltipText={t("home.stats.tooltips.wpm")}
      />
      <StatCard
        icon={Type}
        label={t("home.stats.words")}
        value={
          <span className="flex items-center gap-1">
            <AnimatedCounter
              value={totalWords}
              formatter={(val) => Math.round(val).toLocaleString()}
            />
            <span className="text-lg">🚀</span>
          </span>
        }
        subtext={getWordEquivalent(totalWords)}
        tooltipText={t("home.stats.tooltips.words")}
      />
      <StatCard
        icon={Clock}
        label={t("home.stats.timeSaved")}
        value={formatTimeSaved(timeSavedMins)}
        subtext={getTimeEquivalent(timeSavedMins)}
        tooltipText={t("home.stats.tooltips.timeSaved")}
      />
    </div>
  );
}
