import { emit, listen } from "@tauri-apps/api/event";
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CancelIcon } from "../components/icons";
import "./RecordingOverlay.css";
import { commands } from "@/bindings";
import i18n, { syncLanguageFromSettings } from "@/i18n";
import { colors } from "@/theme";
import { logInfo, logWarn } from "@/utils/logging";
import { AudioAGC } from "@/utils/audioAGC";
import { getLanguageDirection } from "@/lib/utils/rtl";

type OverlayState =
  | "recording"
  | "transcribing"
  | "processing"
  | "connecting"
  | "cancelling"
  | "correcting";

interface CorrectionResult {
  original: string;
  corrected: string;
  has_changes: boolean;
}

type UndoOverlayKind =
  | "feedback"
  | "discoverability_hint";

interface UndoOverlayEventPayload {
  kind: UndoOverlayKind;
  code: string;
  shortcut?: string | null;
}

interface OverlayClientRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface OverlayInteractionRegionsPayload {
  overlayVisible: boolean;
  messageLaneRect: OverlayClientRect | null;
  actionRects: OverlayClientRect[];
}

interface OverlayHoverTooltipState {
  text: string;
  x: number;
  y: number;
  placement: "above" | "below" | "inline" | "left";
}

// SVG dimensions and border radius (constants)
const SVG_WIDTH = 234;
const SVG_HEIGHT = 40;
const SVG_RX = 20; // border radius
const SVG_STROKE_WIDTH = 2;
const SVG_PATH_LENGTH = 100;
const UNDO_FEEDBACK_AUTO_DISMISS_MS = 1000;
const UNDO_DISCOVERABILITY_AUTO_DISMISS_MS = 12000;
const UNDO_FEEDBACK_CLEAR_AFTER_HIDE_MS = 180;
const DEFAULT_SLOT_WIDTH_PX = 24;
const MARQUEE_GAP_PX = 28;
const MARQUEE_SPEED_PX_PER_SEC = 38;
const TOOLTIP_EDGE_PADDING_PX = 6;
const TOOLTIP_VERTICAL_GAP_PX = 6;
const TOOLTIP_HORIZONTAL_GAP_PX = 0;
const TOOLTIP_ESTIMATED_HEIGHT_PX = 24;

const toClientRect = (element: HTMLElement | null): OverlayClientRect | null => {
  if (!element) {
    return null;
  }

  const rect = element.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) {
    return null;
  }

  return {
    x: rect.left,
    y: rect.top,
    width: rect.width,
    height: rect.height,
  };
};

const expandClientRect = (
  rect: OverlayClientRect | null,
  paddingPx: number,
): OverlayClientRect | null => {
  if (!rect) {
    return null;
  }

  return {
    x: rect.x - paddingPx,
    y: rect.y - paddingPx,
    width: rect.width + paddingPx * 2,
    height: rect.height + paddingPx * 2,
  };
};

// Pre-computed SVG path for the rounded rect (starts at top-center, clockwise)
const ROUNDED_RECT_PATH = (() => {
  const x = SVG_STROKE_WIDTH / 2;
  const y = SVG_STROKE_WIDTH / 2;
  const w = SVG_WIDTH - SVG_STROKE_WIDTH;
  const h = SVG_HEIGHT - SVG_STROKE_WIDTH;
  const r = SVG_RX - SVG_STROKE_WIDTH / 2;
  const topCenterX = x + w / 2;

  // Start at top-center, draw clockwise around the rounded rect
  return `M ${topCenterX},${y} L ${x + w - r},${y} Q ${x + w},${y} ${x + w},${y + r} L ${x + w},${y + h - r} Q ${x + w},${y + h} ${x + w - r},${y + h} L ${x + r},${y + h} Q ${x},${y + h} ${x},${y + h - r} L ${x},${y + r} Q ${x},${y} ${x + r},${y} L ${topCenterX},${y}`;
})();

const RecordingOverlay: React.FC = () => {
  const { t } = useTranslation();
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>("recording");
  const [levels, setLevels] = useState<number[]>(Array(16).fill(0));
  const [correctionData, setCorrectionData] = useState<CorrectionResult | null>(
    null,
  );
  const [undoCard, setUndoCard] = useState<UndoOverlayEventPayload | null>(
    null,
  );
  const undoCardRef = useRef<UndoOverlayEventPayload | null>(null);
  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  const agcRef = useRef(new AudioAGC());
  const undoDismissTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );
  const undoCardClearTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );
  const direction = getLanguageDirection(i18n.language);

  // Recording time state
  const [elapsedSecs, setElapsedSecs] = useState(0);
  const [maxSecs, setMaxSecs] = useState(480); // Default 8 min

  // Calculate progress (0 to 1, where 1 = full, 0 = empty)
  const progress = maxSecs > 0 ? Math.max(0, 1 - elapsedSecs / maxSecs) : 1;

  // Memoize dashOffset calculation
  const dashOffset = useMemo(
    () => SVG_PATH_LENGTH * (1 - progress),
    [progress],
  );

  const isDiscoverabilityHint = (payload: UndoOverlayEventPayload) =>
    payload.kind === "discoverability_hint";
  const marqueeViewportRef = useRef<HTMLDivElement | null>(null);
  const marqueeTextRef = useRef<HTMLSpanElement | null>(null);
  const cancelActionRef = useRef<HTMLButtonElement | null>(null);
  const [marqueeMetrics, setMarqueeMetrics] = useState<{
    overflow: boolean;
    distancePx: number;
    durationSec: number;
  }>({
    overflow: false,
    distancePx: 0,
    durationSec: 0,
  });
  const [isMarqueePaused, setIsMarqueePaused] = useState(false);
  const [pointerLaneHover, setPointerLaneHover] = useState(false);
  const [nativeLaneHover, setNativeLaneHover] = useState(false);
  const [nativeCursorIntent, setNativeCursorIntent] = useState<
    "default" | "pointer"
  >("default");
  const [isPointerOverAction, setIsPointerOverAction] = useState(false);
  const [hoverTooltip, setHoverTooltip] =
    useState<OverlayHoverTooltipState | null>(null);

  const getActionTooltipPosition = useCallback(
    (
      element: HTMLElement,
      text: string,
      placementPreference: "auto" | "left" = "auto",
    ): OverlayHoverTooltipState => {
      const rect = element.getBoundingClientRect();
      const viewportWidth = window.innerWidth || SVG_WIDTH;
      const viewportHeight = window.innerHeight || SVG_HEIGHT;
      const estimatedWidth = Math.max(
        44,
        Math.min(180, text.length * 6.2 + 16),
      );
      const halfEstimatedWidth = estimatedWidth / 2;
      const anchorX = rect.left + rect.width / 2;
      const x = Math.min(
        viewportWidth - TOOLTIP_EDGE_PADDING_PX - halfEstimatedWidth,
        Math.max(
          TOOLTIP_EDGE_PADDING_PX + halfEstimatedWidth,
          anchorX,
        ),
      );

      if (placementPreference === "left") {
        const anchorLeftX = rect.left - TOOLTIP_HORIZONTAL_GAP_PX;
        const canPlaceLeft =
          anchorLeftX - estimatedWidth >= TOOLTIP_EDGE_PADDING_PX;
        if (canPlaceLeft) {
          const centeredY = Math.max(
            TOOLTIP_EDGE_PADDING_PX + TOOLTIP_ESTIMATED_HEIGHT_PX / 2,
            Math.min(
              viewportHeight -
                TOOLTIP_EDGE_PADDING_PX -
                TOOLTIP_ESTIMATED_HEIGHT_PX / 2,
              rect.top + rect.height / 2,
            ),
          );
          return {
            text,
            x: anchorLeftX,
            y: centeredY,
            placement: "left",
          };
        }
      }

      const aboveY = rect.top - TOOLTIP_VERTICAL_GAP_PX;
      const belowY = rect.bottom + TOOLTIP_VERTICAL_GAP_PX;
      const canPlaceAbove =
        aboveY - TOOLTIP_ESTIMATED_HEIGHT_PX >= TOOLTIP_EDGE_PADDING_PX;
      const canPlaceBelow =
        belowY + TOOLTIP_ESTIMATED_HEIGHT_PX <=
        viewportHeight - TOOLTIP_EDGE_PADDING_PX;

      if (canPlaceAbove) {
        return { text, x, y: aboveY, placement: "above" };
      }

      if (canPlaceBelow) {
        return { text, x, y: belowY, placement: "below" };
      }

      const inlineY = Math.max(
        TOOLTIP_EDGE_PADDING_PX,
        Math.min(
          viewportHeight - TOOLTIP_ESTIMATED_HEIGHT_PX - TOOLTIP_EDGE_PADDING_PX,
          rect.top + rect.height / 2 - TOOLTIP_ESTIMATED_HEIGHT_PX / 2,
        ),
      );
      return { text, x, y: inlineY, placement: "inline" };
    },
    [],
  );

  const showActionTooltip = useCallback(
    (
      event: React.PointerEvent<HTMLButtonElement>,
      text: string,
      placementPreference: "auto" | "left" = "auto",
    ) => {
      setHoverTooltip(
        getActionTooltipPosition(
          event.currentTarget,
          text,
          placementPreference,
        ),
      );
    },
    [getActionTooltipPosition],
  );

  const hideActionTooltip = useCallback(() => {
    setHoverTooltip(null);
  }, []);

  const clearUndoDismissTimer = () => {
    if (undoDismissTimerRef.current) {
      clearTimeout(undoDismissTimerRef.current);
      undoDismissTimerRef.current = null;
    }
  };

  const clearUndoCardClearTimer = () => {
    if (undoCardClearTimerRef.current) {
      clearTimeout(undoCardClearTimerRef.current);
      undoCardClearTimerRef.current = null;
    }
  };

  const scheduleUndoAutoDismiss = (payload: UndoOverlayEventPayload) => {
    clearUndoDismissTimer();
    clearUndoCardClearTimer();
    const autoDismissMs = isDiscoverabilityHint(payload)
      ? UNDO_DISCOVERABILITY_AUTO_DISMISS_MS
      : UNDO_FEEDBACK_AUTO_DISMISS_MS;

    undoDismissTimerRef.current = setTimeout(() => {
      clearUndoDismissTimer();
      const activeCard = undoCardRef.current;
      if (!activeCard) {
        return;
      }

      setIsVisible(false);
      commands.undoOverlayCardDismissed().catch((error) => {
        logWarn(
          `Failed to release overlay interaction: ${error}`,
          "fe-overlay",
        );
      });

      // Keep the feedback text through fade-out, then clear card state.
      undoCardClearTimerRef.current = setTimeout(() => {
        clearUndoCardClearTimer();
        setUndoCard((current) => {
          if (!current) {
            return current;
          }
          if (current !== activeCard) {
            return current;
          }
          return null;
        });
      }, UNDO_FEEDBACK_CLEAR_AFTER_HIDE_MS);
    }, autoDismissMs);
  };

  const showUndoCard = (payload: UndoOverlayEventPayload) => {
    clearUndoCardClearTimer();
    setUndoCard(payload);
    setIsVisible(true);
    commands.undoOverlayCardPresented().catch((error) => {
      logWarn(
        `Failed to set overlay interaction for undo card: ${error}`,
        "fe-overlay",
      );
    });
    scheduleUndoAutoDismiss(payload);
  };

  const undoCardMessage = useMemo(() => {
    if (!undoCard) return "";
    if (undoCard.kind === "discoverability_hint") {
      return t(
        "overlay.undo.discoverability.hint",
        "Tip: Press {{shortcut}} to undo your last transcript within 2 minutes.",
        {
          shortcut:
            undoCard.shortcut ??
            t(
              "settings.general.shortcut.bindings.undo_last_transcript.name",
              "Undo last transcript",
            ),
        },
      );
    }
    const feedbackMap: Record<string, string> = {
      undo_success: t("overlay.undo.feedback.success", "Undo applied"),
      undo_failed: t("overlay.undo.feedback.failed", "Undo failed"),
      undo_recording_canceled: t(
        "overlay.undo.feedback.recordingCanceled",
        "Recording canceled",
      ),
      undo_processing_canceled: t(
        "overlay.undo.feedback.processingCanceled",
        "Processing canceled",
      ),
      undo_noop_empty: t(
        "overlay.undo.feedback.nothingToUndo",
        "Nothing to undo",
      ),
      undo_noop_expired: t("overlay.undo.feedback.expired", "Undo expired"),
    };
    return (
      feedbackMap[undoCard.code] ??
      t("overlay.undo.feedback.success", "Undo applied")
    );
  }, [t, undoCard]);

  const stateMessage = useMemo(() => {
    if (state === "transcribing") {
      return t("overlay.transcribing");
    }
    if (state === "processing") {
      return t("overlay.processing");
    }
    if (state === "connecting") {
      return t("overlay.starting", "Starting microphone...");
    }
    if (state === "cancelling") {
      return t("overlay.cancelling", "Cancelling...");
    }
    if (state === "correcting" && !correctionData) {
      return t("overlay.correcting");
    }
    return "";
  }, [correctionData, state, t]);

  const overlayMessageText = undoCard ? undoCardMessage : stateMessage;
  const discoverabilityActive = undoCard?.kind === "discoverability_hint";
  const marqueeEligible = discoverabilityActive;

  const usesCancelSlot =
    !undoCard &&
    (state === "recording" ||
      state === "transcribing" ||
      state === "processing" ||
      state === "connecting" ||
      state === "cancelling");
  const usesCorrectionPlaceholder = !undoCard && state === "correcting";
  const pointerCursorActive =
    nativeCursorIntent === "pointer" || isPointerOverAction;

  const recomputeMarquee = useCallback(() => {
    const viewport = marqueeViewportRef.current;
    const text = marqueeTextRef.current;
    if (!marqueeEligible || !viewport || !text || !overlayMessageText) {
      setMarqueeMetrics((current) =>
        current.overflow || current.distancePx !== 0 || current.durationSec !== 0
          ? { overflow: false, distancePx: 0, durationSec: 0 }
          : current,
      );
      return;
    }

    const viewportWidth = viewport.clientWidth;
    const textWidth = text.scrollWidth;
    const overflow = textWidth > viewportWidth + 1;

    if (!overflow) {
      setMarqueeMetrics((current) =>
        current.overflow || current.distancePx !== 0 || current.durationSec !== 0
          ? { overflow: false, distancePx: 0, durationSec: 0 }
          : current,
      );
      return;
    }

    const distancePx = textWidth + MARQUEE_GAP_PX;
    const durationSec = Math.max(5, distancePx / MARQUEE_SPEED_PX_PER_SEC);
    setMarqueeMetrics((current) => {
      if (
        current.overflow === true &&
        current.distancePx === distancePx &&
        current.durationSec === durationSec
      ) {
        return current;
      }
      return {
        overflow: true,
        distancePx,
        durationSec,
      };
    });
  }, [marqueeEligible, overlayMessageText]);

  useEffect(() => {
    recomputeMarquee();
  }, [overlayMessageText, isVisible, recomputeMarquee]);

  const marqueeShouldPause =
    isVisible &&
    marqueeEligible &&
    marqueeMetrics.overflow &&
    (pointerLaneHover || nativeLaneHover);

  useEffect(() => {
    if (isMarqueePaused === marqueeShouldPause) {
      return;
    }
    setIsMarqueePaused(marqueeShouldPause);
  }, [
    isMarqueePaused,
    marqueeShouldPause,
  ]);

  useEffect(() => {
    const viewport = marqueeViewportRef.current;
    if (!viewport || typeof ResizeObserver === "undefined") {
      return;
    }

    const observer = new ResizeObserver(() => {
      recomputeMarquee();
    });
    observer.observe(viewport);
    return () => {
      observer.disconnect();
    };
  }, [recomputeMarquee]);

  const marqueeStyle =
    marqueeEligible && marqueeMetrics.overflow && overlayMessageText
      ? ({
          "--marquee-distance": `-${marqueeMetrics.distancePx}px`,
          "--marquee-duration": `${marqueeMetrics.durationSec}s`,
        } as React.CSSProperties)
      : undefined;

  const publishInteractionRegions = useCallback(() => {
    const actionPaddingPx = 12;
    const regions: OverlayInteractionRegionsPayload = {
      overlayVisible: isVisible,
      messageLaneRect: isVisible ? toClientRect(marqueeViewportRef.current) : null,
      actionRects: isVisible
        ? [
            state !== "cancelling"
              ? expandClientRect(
                  toClientRect(cancelActionRef.current),
                  actionPaddingPx,
                )
              : null,
          ].filter((rect): rect is OverlayClientRect => rect !== null)
        : [],
    };

    commands.overlayUpdateInteractionRegions(regions).catch((error) => {
      logWarn(
        `RecordingOverlay: Failed to publish interaction regions: ${error}`,
        "fe-overlay",
      );
    });
  }, [isVisible, state]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      publishInteractionRegions();
    });
    return () => {
      window.cancelAnimationFrame(frame);
    };
  }, [
    discoverabilityActive,
    isVisible,
    marqueeMetrics.overflow,
    overlayMessageText,
    publishInteractionRegions,
    state,
    usesCancelSlot,
  ]);

  useEffect(() => {
    const onResize = () => {
      publishInteractionRegions();
    };
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
    };
  }, [publishInteractionRegions]);

  useEffect(() => {
    if (!isVisible) {
      return;
    }
    const intervalId = window.setInterval(() => {
      publishInteractionRegions();
    }, 250);
    return () => {
      window.clearInterval(intervalId);
    };
  }, [isVisible, publishInteractionRegions]);

  useEffect(() => {
    undoCardRef.current = undoCard;
  }, [undoCard]);

  useEffect(() => {
    // React StrictMode-safe pattern: https://react.dev/learn/synchronizing-with-effects#fetching-data
    // Use `ignore` flag to prevent state updates after cleanup
    let ignore = false;
    const cleanupFns: Array<() => void> = [];

    async function setupListeners() {
      // Listen for show-overlay event from Rust
      const unlistenShow = await listen("show-overlay", async (event) => {
        if (ignore) return; // Ignore if cleanup already ran
        if (undoCardRef.current) {
          commands.undoOverlayCardDismissed().catch((error) => {
            logWarn(
              `Failed to release overlay interaction during operation state switch: ${error}`,
              "fe-overlay",
            );
          });
        }
        clearUndoDismissTimer();
        clearUndoCardClearTimer();
        setUndoCard(null);
        setPointerLaneHover(false);
        setNativeLaneHover(false);
        setNativeCursorIntent("default");
        setIsPointerOverAction(false);
        setHoverTooltip(null);
        const overlayState = event.payload as OverlayState;
        setState(overlayState);
        setIsVisible(true);

        // Reset time when showing overlay in recording state
        if (overlayState === "recording") {
          setElapsedSecs(0);
          agcRef.current.reset();
        }

        // Sync language from settings (fire-and-forget, don't await/block UI)
        syncLanguageFromSettings().catch((err) => {
          logWarn(
            `RecordingOverlay: Language sync failed: ${err}`,
            "fe-overlay",
          );
        });
      });

      // If cleanup ran while awaiting, unsubscribe immediately
      if (ignore) {
        unlistenShow();
        return;
      }
      cleanupFns.push(unlistenShow);

      // Listen for hide-overlay event from Rust
      const unlistenHide = await listen("hide-overlay", () => {
        if (ignore) return;
        if (undoCardRef.current) {
          commands.undoOverlayCardDismissed().catch((error) => {
            logWarn(
              `Failed to release overlay interaction during hide: ${error}`,
              "fe-overlay",
            );
          });
        }
        clearUndoDismissTimer();
        clearUndoCardClearTimer();
        setUndoCard(null);
        setPointerLaneHover(false);
        setNativeLaneHover(false);
        setNativeCursorIntent("default");
        setIsPointerOverAction(false);
        setHoverTooltip(null);
        setIsVisible(false);
        setElapsedSecs(0);
        setCorrectionData(null);

        // Wait for fade-out animation (300ms) to complete before resetting state
        // This prevents the overlay from switching back to "recording" (audio bars)
        // while it is still visible fading out.
        setTimeout(() => {
          if (!ignore) {
            setState("recording");
          }
        }, 350);
      });

      if (ignore) {
        unlistenHide();
        return;
      }
      cleanupFns.push(unlistenHide);

      // Listen for mic-level updates
      const unlistenLevel = await listen<number[]>("mic-level", (event) => {
        if (ignore) return;
        const newLevels = event.payload as number[];

        // Apply smoothing to reduce jitter
        const smoothed = smoothedLevelsRef.current.map((prev, i) => {
          const target = newLevels[i] || 0;
          return prev * 0.7 + target * 0.3; // Smooth transition
        });

        smoothedLevelsRef.current = smoothed;

        // Apply AGC normalization
        const normalized = agcRef.current.process(smoothed.slice(0, 16));
        setLevels(normalized);
      });

      if (ignore) {
        unlistenLevel();
        return;
      }
      cleanupFns.push(unlistenLevel);

      // Listen for recording time updates
      const unlistenTime = await listen<[number, number]>(
        "recording-time",
        (event) => {
          if (ignore) return;
          const [elapsed, max] = event.payload;
          setElapsedSecs(elapsed);
          setMaxSecs(max);
        },
      );

      if (ignore) {
        unlistenTime();
        return;
      }
      cleanupFns.push(unlistenTime);

      // Listen for correction result
      const unlistenCorrection = await listen<CorrectionResult>(
        "correction-result",
        (event) => {
          if (ignore) return;
          setCorrectionData(event.payload);
        },
      );

      if (ignore) {
        unlistenCorrection();
        return;
      }
      cleanupFns.push(unlistenCorrection);

      const unlistenUndoOverlay = await listen<UndoOverlayEventPayload>(
        "undo-overlay-event",
        (event) => {
          if (ignore) return;

          const payload = event.payload;

          if (payload.kind === "discoverability_hint") {
            logInfo("event=undo_discoverability_hint_shown channel=overlay", "fe-overlay");
            commands
              .undoMarkDiscoverabilityHintSeen()
              .then(() => {
                logInfo(
                  "event=undo_discoverability_hint_seen_marked channel=overlay",
                  "fe-overlay",
                );
              })
              .catch((error) => {
                logWarn(
                  `event=undo_discoverability_hint_seen_mark_failed channel=overlay error=${error}`,
                  "fe-overlay",
                );
              });
          }
          showUndoCard(payload);
        },
      );

      if (ignore) {
        unlistenUndoOverlay();
        return;
      }
      cleanupFns.push(unlistenUndoOverlay);

      const unlistenHoverEnter = await listen("overlay-hover-enter", () => {
        if (ignore) return;
        setNativeLaneHover(true);
      });

      if (ignore) {
        unlistenHoverEnter();
        return;
      }
      cleanupFns.push(unlistenHoverEnter);

      const unlistenHoverLeave = await listen("overlay-hover-leave", () => {
        if (ignore) return;
        setNativeLaneHover(false);
      });

      if (ignore) {
        unlistenHoverLeave();
        return;
      }
      cleanupFns.push(unlistenHoverLeave);

      const unlistenCursorIntent = await listen<string>(
        "overlay-cursor-intent",
        (event) => {
          if (ignore) return;
          const intent = event.payload === "pointer" ? "pointer" : "default";
          setNativeCursorIntent(intent);
        },
      );

      if (ignore) {
        unlistenCursorIntent();
        return;
      }
      cleanupFns.push(unlistenCursorIntent);

      // Signal to Rust that the overlay is ready to receive events
      // This prevents the race condition where events are emitted before listeners are registered
      await emit("overlay-ready");
    }

    setupListeners();

    // Cleanup function - called on unmount or before re-running effect
    return () => {
      ignore = true; // Prevent any pending async operations from updating state
      clearUndoDismissTimer();
      clearUndoCardClearTimer();
      setPointerLaneHover(false);
      setNativeLaneHover(false);
      setNativeCursorIntent("default");
      setIsPointerOverAction(false);
      setHoverTooltip(null);
      cleanupFns.forEach((fn) => fn()); // Unsubscribe all listeners
    };
  }, []);

  // NOTE: Tab/Esc keyboard handling for correction accept/dismiss is done
  // in Rust (fn_key_monitor.rs CGEventTap) because the overlay panel is
  // no_activate and cannot receive keyboard events directly.

  // Left slot is only needed to keep symmetric layouts (recording bars/correction views).
  // For status-text states with a right-side cancel button, remove left slot to maximize text room.
  const shouldKeepSymmetricLeftSlot = state === "recording" || state === "correcting";
  const sideSlotWidths = {
    left:
      !discoverabilityActive && shouldKeepSymmetricLeftSlot
        ? DEFAULT_SLOT_WIDTH_PX
        : 0,
    right:
      !discoverabilityActive && (usesCancelSlot || usesCorrectionPlaceholder)
        ? DEFAULT_SLOT_WIDTH_PX
        : 0,
  };

  const cancelTooltipText = t("overlay.cancel", "Cancel");

  useEffect(() => {
    if (!isVisible || !usesCancelSlot || !cancelActionRef.current) {
      return;
    }

    if (nativeCursorIntent === "pointer" && !isPointerOverAction) {
      setHoverTooltip(
        getActionTooltipPosition(
          cancelActionRef.current,
          cancelTooltipText,
          "left",
        ),
      );
      return;
    }

    if (
      nativeCursorIntent === "default" &&
      !isPointerOverAction &&
      hoverTooltip?.text === cancelTooltipText
    ) {
      setHoverTooltip(null);
    }
  }, [
    cancelTooltipText,
    hoverTooltip?.text,
    isPointerOverAction,
    isVisible,
    nativeCursorIntent,
    getActionTooltipPosition,
    usesCancelSlot,
  ]);

  const renderMessageLane = (
    message: string,
    ariaLive = false,
    tone: "status" | "undo" = "status",
    enableMarquee = false,
  ) => {
    const laneMarqueeActive = enableMarquee && marqueeMetrics.overflow;
    return (
      <div
        className="connecting-container"
        role={ariaLive ? "status" : undefined}
        aria-live={ariaLive ? "polite" : undefined}
      >
        <div
          ref={marqueeViewportRef}
          className={`overlay-message-viewport ${laneMarqueeActive ? "marquee-active" : ""}`}
          tabIndex={laneMarqueeActive ? 0 : -1}
          onPointerEnter={() => {
            setPointerLaneHover(true);
          }}
          onPointerLeave={() => {
            setPointerLaneHover(false);
          }}
        >
          <div
            className={`overlay-message-track ${laneMarqueeActive ? "marquee-running" : ""} ${laneMarqueeActive && isMarqueePaused ? "marquee-paused" : ""}`}
            style={laneMarqueeActive ? marqueeStyle : undefined}
          >
            <span
              ref={marqueeTextRef}
              className={`${tone === "undo" ? "undo-message-text" : "connecting-text"} overlay-message-text`}
            >
              {message}
            </span>
            {laneMarqueeActive && (
              <span
                aria-hidden
                className={`${tone === "undo" ? "undo-message-text" : "connecting-text"} overlay-message-text overlay-message-text-clone`}
              >
                {message}
              </span>
            )}
          </div>
        </div>
      </div>
    );
  };

  return (
    <>
      <div
        className={`recording-overlay-wrapper ${isVisible ? "fade-in" : "fade-out"} ${pointerCursorActive ? "overlay-pointer-intent" : ""}`}
        dir={direction}
        onPointerLeave={() => {
          setPointerLaneHover(false);
          setIsPointerOverAction(false);
          hideActionTooltip();
        }}
      >
      {/* SVG countdown border - uniform animation along perimeter */}
      {state === "recording" && !undoCard && (
        <svg
          className="countdown-border"
          width={SVG_WIDTH}
          height={SVG_HEIGHT}
          viewBox={`0 0 ${SVG_WIDTH} ${SVG_HEIGHT}`}
        >
          <path
            d={ROUNDED_RECT_PATH}
            fill="none"
            stroke={`var(--color-border, ${colors.border})`}
            strokeWidth={SVG_STROKE_WIDTH}
            pathLength={SVG_PATH_LENGTH}
            strokeDasharray={SVG_PATH_LENGTH}
            strokeDashoffset={dashOffset}
            strokeLinecap="round"
          />
        </svg>
      )}

      <div
        className={`recording-overlay-inner ${discoverabilityActive ? "discoverability-layout" : ""}`}
        style={
          {
            "--overlay-left-slot-width": `${sideSlotWidths.left}px`,
            "--overlay-right-slot-width": `${sideSlotWidths.right}px`,
          } as React.CSSProperties
        }
      >
        <div className="overlay-left">
          {sideSlotWidths.left > 0 ? (
            <div className="overlay-slot-placeholder" aria-hidden />
          ) : null}
        </div>

        <div className="overlay-middle">
          {undoCard ? (
            renderMessageLane(undoCardMessage, true, "undo", discoverabilityActive)
          ) : (
            <>
              {state === "recording" && (
                <div className="bars-container">
                  {levels.map((v, i) => (
                    <div
                      key={i}
                      className="bar"
                      style={{
                        // Height scales from 4px to 20px based on normalized level (0-1)
                        height: `${4 + v * 16}px`,
                        transition:
                          "height 60ms ease-out, opacity 120ms ease-out",
                        opacity: Math.max(0.4, Math.min(1, 0.4 + v * 0.6)),
                      }}
                    />
                  ))}
                </div>
              )}
              {state === "transcribing" && (
                renderMessageLane(overlayMessageText)
              )}
              {state === "processing" && (
                renderMessageLane(overlayMessageText)
              )}
              {state === "connecting" && (
                renderMessageLane(overlayMessageText)
              )}
              {state === "cancelling" && (
                renderMessageLane(overlayMessageText)
              )}
              {state === "correcting" && correctionData && (
                <div className="correction-container">
                  <div className="correction-text">
                    <span className="correction-original">
                      {correctionData.original}
                    </span>
                    <span className="correction-arrow">→</span>
                    <span className="correction-corrected">
                      {correctionData.corrected}
                    </span>
                  </div>
                  <div className="correction-hint">
                    {t("overlay.correctionHint")}
                  </div>
                </div>
              )}
              {state === "correcting" && !correctionData && (
                renderMessageLane(overlayMessageText)
              )}
            </>
          )}
        </div>

        <div className="overlay-right">
          {usesCancelSlot && (
            <button
              type="button"
              ref={cancelActionRef}
              className={`cancel-button ${state === "cancelling" ? "disabled" : ""}`}
              onClick={() => {
                setIsPointerOverAction(false);
                hideActionTooltip();
                if (state !== "cancelling") {
                  commands.cancelOperation();
                }
              }}
              onPointerEnter={(event) => {
                setIsPointerOverAction(true);
                showActionTooltip(event, cancelTooltipText, "left");
              }}
              onPointerLeave={() => {
                setIsPointerOverAction(false);
                hideActionTooltip();
              }}
              disabled={state === "cancelling"}
              aria-label={cancelTooltipText}
              title={cancelTooltipText}
            >
              {state !== "cancelling" && (
                <CancelIcon width={20} height={20} color="rgba(255, 255, 255, 0.88)" />
              )}
            </button>
          )}
          {usesCorrectionPlaceholder && <div className="overlay-slot-placeholder" aria-hidden />}
        </div>
      </div>
      </div>
      {hoverTooltip && isVisible && (
        <div
          className={`overlay-hover-tooltip tooltip-placement-${hoverTooltip.placement}`}
          role="tooltip"
          style={{ left: `${hoverTooltip.x}px`, top: `${hoverTooltip.y}px` }}
        >
          {hoverTooltip.text}
        </div>
      )}
    </>
  );
};

export default RecordingOverlay;
