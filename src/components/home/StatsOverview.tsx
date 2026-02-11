import React, { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Card, CardContent } from "@/components/shared/ui/card";
import { Zap, Clock, Type, Trophy, Info, Sparkles } from "lucide-react";
import { AnimatedCounter } from "@/components/shared/AnimatedCounter";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/shared/ui/tooltip";
import { listen } from "@tauri-apps/api/event";

interface Stats {
  total_words: number;
  total_duration_minutes: number;
  wpm: number;
  time_saved_minutes: number;
  streak_days: number;
  faster_than_typing_percentage: number;
  total_filler_words_removed: number;
  filler_filter_active: boolean;
}

type SmartTileFace = "streak" | "filler";

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
            key={focusKey}
            duration={2000}
            formatter={(val) => Math.round(val).toString()}
          />
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
            <AnimatedCounter value={mins} duration={2000} key={`m-${focusKey}`} />
            {"m "}
            <AnimatedCounter value={secs} duration={2000} key={`s-${focusKey}`} />
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
          <AnimatedCounter value={days} duration={3000} key={`d-${focusKey}`} />
          {"d "}
          <AnimatedCounter value={hours} duration={3000} key={`h-${focusKey}`} />
          {"h "}
          <AnimatedCounter value={mins} duration={3000} key={`m-${focusKey}`} />
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
          <AnimatedCounter value={hours} duration={3000} key={`h-${focusKey}`} />
          {"h "}
          <AnimatedCounter value={mins} duration={3000} key={`m-${focusKey}`} />
          {"m "}
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="text-lg">⏱️</span>
        </>
      );
    }
    
    // Fallback using standard animation
    return (
      <>
        <AnimatedCounter value={totalMins} duration={2000} key={focusKey} />{" "}
        {totalMins <= 1 ? "min" : "mins"} 
        {/* eslint-disable-next-line i18next/no-literal-string */}
        <span className="text-lg">⏱️</span>
      </>
    );
  };

  const totalFillerWordsRemoved = stats?.total_filler_words_removed ?? 0;
  const fillerFilterActive = stats?.filler_filter_active ?? false;

  // Smart tile priority logic:
  // Lock to streak face ONLY if filler stats have nothing to show:
  //   - Filler filter is off (no point showing filler stats)
  //   - No filler words have been removed yet (nothing to celebrate)
  // When rotation IS enabled, the streak face always shows FIRST on focus,
  // so users still see streak encouragement/milestones before the tile rotates.
  const shouldLockStreak =
    !fillerFilterActive ||
    totalFillerWordsRemoved === 0;

  // Debug logging for smart tile state
  /* console.debug("[SmartTile]", {
    shouldLockStreak,
    fillerFilterActive,
    totalFillerWordsRemoved,
    streakDays,
    currentStats: stats,
  }); */

  const [currentFace, setCurrentFace] = useState<SmartTileFace>("streak");
  const [isAnimating, setIsAnimating] = useState(false);
  const [focusKey, setFocusKey] = useState(0);
  const rotationTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Ref to read currentFace inside callbacks without adding it as a dependency
  const currentFaceRef = useRef<SmartTileFace>(currentFace);
  currentFaceRef.current = currentFace;

  // Rotation handler: show streak first, then animate to filler after 2s
  // Uses currentFaceRef to avoid re-creating the callback on every face change,
  // which would cause the focus event listener to re-register unnecessarily.
  const handleFocusRotation = useCallback(() => {
    // Trigger stat counters to re-animate
    setFocusKey((prev) => prev + 1);

    if (shouldLockStreak) {
      setCurrentFace("streak");
      setIsAnimating(false);
      return;
    }

    // Clear existing timers
    if (rotationTimerRef.current) clearTimeout(rotationTimerRef.current);

    const startRotationSequence = () => {
      rotationTimerRef.current = setTimeout(() => {
        setIsAnimating(true); // Fade out streak
        setTimeout(() => {
          setCurrentFace("filler");
          setIsAnimating(false); // Filler fades in (via animate-in class)
        }, 400);
      }, 2000);
    };

    if (currentFaceRef.current === "filler") {
      // Reverse animation: Animate filler OUT, then Streak IN
      setIsAnimating(true); 
      setTimeout(() => {
        setCurrentFace("streak");
        setIsAnimating(false);
        // Then start the normal rotation sequence
        startRotationSequence();
      }, 400);
    } else {
      // Already on streak (or mounting). ensure we are on streak and start sequence.
      setCurrentFace("streak");
      setIsAnimating(false);
      startRotationSequence();
    }
  }, [shouldLockStreak]);

  // Listen for app focus events with debounce
  useEffect(() => {
    console.debug("[SmartTile] Setting up focus listener");
    const unlisten = listen("tauri://focus", () => {
      console.debug("[SmartTile] tauri://focus event received");
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = setTimeout(() => {
        handleFocusRotation();
      }, 300);
    });

    return () => {
      unlisten.then((fn) => fn());
      if (rotationTimerRef.current) clearTimeout(rotationTimerRef.current);
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    };
  }, [handleFocusRotation]);

  // Trigger rotation on initial mount
  useEffect(() => {
    const timer = setTimeout(() => {
      handleFocusRotation();
    }, 500); // Small delay to allow initial render/animations to settle
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Reset to streak face when lock conditions change
  useEffect(() => {
    if (shouldLockStreak) {
      setCurrentFace("streak");
      setIsAnimating(false);
      if (rotationTimerRef.current) clearTimeout(rotationTimerRef.current);
    }
  }, [shouldLockStreak]);

  // Filler word subtext
  const getFillerSubtext = (): string => {
    if (totalFillerWordsRemoved === 0) {
      return t("home.stats.fillerWords.zero");
    }
    return t("home.stats.fillerWords.encouragement");
  };

  // Smart tile content renderer
  const renderSmartTile = () => {
    const isStreak = currentFace === "streak";

    const streakContent = (
      <div className={`flex flex-col gap-2 transition-all duration-400 animate-in fade-in slide-in-from-bottom-2 ${
        isAnimating && isStreak ? "opacity-0 -translate-y-2" : "opacity-100 translate-y-0"
      }`}>
        <div className="flex items-center gap-2 text-muted-foreground text-sm font-medium uppercase tracking-wider">
          <Zap size={16} />
          {t("home.stats.streak")}
          <TooltipProvider>
            <Tooltip delayDuration={300}>
              <TooltipTrigger asChild>
                <div className="cursor-help transition-opacity hover:opacity-80">
                  <Info size={14} className="text-muted-foreground/70" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-[250px] text-xs">
                {t("home.stats.tooltips.streak")}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          {!shouldLockStreak && (
            <div className="ml-auto flex gap-0.5">
              <div className="w-1.5 h-1.5 rounded-full bg-primary" />
              <div className="w-1.5 h-1.5 rounded-full bg-muted-foreground/30" />
            </div>
          )}
        </div>
        <div className="text-3xl font-bold">
          {streakDays}{" "}
          {streakDays === 1 ? t("home.stats.day") : t("home.stats.days")}{" "}
          <span className="text-lg">🔥</span>
        </div>
        <div className="text-sm text-muted-foreground">
          {getStreakEncouragement(streakDays)}
        </div>
      </div>
    );

    const fillerContent = (
      <div className={`flex flex-col gap-2 transition-all duration-400 animate-in fade-in slide-in-from-bottom-2 ${
        isAnimating && !isStreak ? "opacity-0 translate-y-2" : "opacity-100 translate-y-0"
      }`}>
        <div className="flex items-center gap-2 text-muted-foreground text-sm font-medium uppercase tracking-wider">
          <Sparkles size={16} />
          {t("home.stats.fillerWords.label")}
          <TooltipProvider>
            <Tooltip delayDuration={300}>
              <TooltipTrigger asChild>
                <div className="cursor-help transition-opacity hover:opacity-80">
                  <Info size={14} className="text-muted-foreground/70" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-[250px] text-xs">
                {t("home.stats.tooltips.fillerWords")}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <div className="ml-auto flex gap-0.5">
            <div className="w-1.5 h-1.5 rounded-full bg-muted-foreground/30" />
            <div className="w-1.5 h-1.5 rounded-full bg-primary" />
          </div>
        </div>
        <div className="text-3xl font-bold">
          <span className="flex items-center gap-1">
            <AnimatedCounter
              value={totalFillerWordsRemoved}
              formatter={(val) => Math.round(val).toLocaleString()}
            />
            <span className="text-lg">✨</span>
          </span>
        </div>
        <div className="text-sm text-muted-foreground">
          {getFillerSubtext()}
        </div>
      </div>
    );

    return (
      <Card className="border-none shadow-sm bg-primary/5">
        <CardContent className="p-6">
          {isStreak ? streakContent : fillerContent}
        </CardContent>
      </Card>
    );
  };

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
      {renderSmartTile()}
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
              key={focusKey}
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
