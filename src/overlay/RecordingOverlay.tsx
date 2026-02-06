import { emit, listen } from "@tauri-apps/api/event";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CancelIcon,
} from "../components/icons";
import "./RecordingOverlay.css";
import { commands } from "@/bindings";
import { syncLanguageFromSettings } from "@/i18n";
import { colors } from "@/theme";
import { logInfo, logWarn, logDebug } from "@/utils/logging";
import { AudioAGC } from "@/utils/audioAGC";


type OverlayState = "recording" | "transcribing" | "processing" | "connecting" | "cancelling";

// SVG dimensions and border radius (constants)
const SVG_WIDTH = 234;
const SVG_HEIGHT = 40;
const SVG_RX = 20; // border radius
const SVG_STROKE_WIDTH = 2;
const SVG_PATH_LENGTH = 100;

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
  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  const agcRef = useRef(new AudioAGC());
  
  // Recording time state
  const [elapsedSecs, setElapsedSecs] = useState(0);
  const [maxSecs, setMaxSecs] = useState(480); // Default 8 min
  
  // Calculate progress (0 to 1, where 1 = full, 0 = empty)
  const progress = maxSecs > 0 ? Math.max(0, 1 - elapsedSecs / maxSecs) : 1;
  
  // Memoize dashOffset calculation
  const dashOffset = useMemo(
    () => SVG_PATH_LENGTH * (1 - progress),
    [progress]
  );

  useEffect(() => {
    // React StrictMode-safe pattern: https://react.dev/learn/synchronizing-with-effects#fetching-data
    // Use `ignore` flag to prevent state updates after cleanup
    let ignore = false;
    const cleanupFns: Array<() => void> = [];
    

    logInfo("RecordingOverlay: Component mounted, setting up event listeners", "fe-overlay");
    
    async function setupListeners() {
      logDebug("RecordingOverlay: Starting to register event listeners", "fe-overlay");
      
      // Listen for show-overlay event from Rust
      const unlistenShow = await listen("show-overlay", async (event) => {
        if (ignore) return;  // Ignore if cleanup already ran
        const now = performance.now();
        const overlayState = event.payload as OverlayState;
        logInfo(`RecordingOverlay: Received show-overlay event: state=${overlayState} at ${now.toFixed(2)}ms`, "fe-overlay");
        
        // Update state IMMEDIATELY before any async operations
        // This ensures the UI shows the correct state as soon as the event arrives
        logInfo(`RecordingOverlay: processing show-overlay event: state=${overlayState}`, "fe-overlay");
        setState(overlayState);
        setIsVisible(true);
        logDebug(`RecordingOverlay: Set isVisible=true, state=${overlayState}`, "fe-overlay");
        
        // Reset time when showing overlay in recording state
        if (overlayState === "recording") {
          setElapsedSecs(0);
          agcRef.current.reset();
        }
        
        // Sync language from settings (fire-and-forget, don't await/block UI)
        syncLanguageFromSettings().catch(err => {
             logWarn(`RecordingOverlay: Language sync failed: ${err}`, "fe-overlay");
        });
        
        // Force a browser reflow/repaint to ensure visibility updates immediately
        // This is a common fix for transparent window painting issues on macOS
        requestAnimationFrame(() => {
             logDebug("RecordingOverlay: Animation frame fired (paint)", "fe-overlay");
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
        logInfo("RecordingOverlay: Received hide-overlay event", "fe-overlay");
        setIsVisible(false);
        setElapsedSecs(0);
        
        // Wait for fade-out animation (300ms) to complete before resetting state
        // This prevents the overlay from switching back to "recording" (audio bars)
        // while it is still visible fading out.
        setTimeout(() => {
          if (!ignore) {
            setState("recording");
            logDebug("RecordingOverlay: Reset state to recording after fade-out", "fe-overlay");
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
      const unlistenTime = await listen<[number, number]>("recording-time", (event) => {
        if (ignore) return;
        const [elapsed, max] = event.payload;
        setElapsedSecs(elapsed);
        setMaxSecs(max);
      });
      
      if (ignore) {
        unlistenTime();
        return;
      }
      cleanupFns.push(unlistenTime);

      logInfo("RecordingOverlay: All event listeners registered successfully", "fe-overlay");
      
      // Signal to Rust that the overlay is ready to receive events
      // This prevents the race condition where events are emitted before listeners are registered
      await emit("overlay-ready");
      logInfo("RecordingOverlay: Emitted overlay-ready signal to Rust", "fe-overlay");
    }

    setupListeners();
    
    // Cleanup function - called on unmount or before re-running effect
    return () => {
      logDebug("RecordingOverlay: Cleanup running, setting ignore=true", "fe-overlay");
      ignore = true;  // Prevent any pending async operations from updating state
      cleanupFns.forEach(fn => fn());  // Unsubscribe all listeners
    };
  }, []);

  const getIcon = () => {
    if (state === "recording") {
      // User reported never seeing the mic icon (or it's redundant). 
      // Using a spacer to balance the Cancel button (24px) on the right.
      return <div style={{ width: 24 }} />;
    } 
    
    // For connecting, transcribing, processing, and cancelling, we return null (no icon)
    // to allow the text to be perfectly centered in the overlay.
    return null;
  };

  return (
    <div 
      className={`recording-overlay-wrapper ${isVisible ? "fade-in" : "fade-out"}`}
    >
      {/* SVG countdown border - uniform animation along perimeter */}
      {state === "recording" && (
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
      
      <div className="recording-overlay-inner">
        <div className="overlay-left">{getIcon()}</div>

        <div className="overlay-middle">
          {state === "recording" && (
            <div className="bars-container">
              {levels.map((v, i) => (
                <div
                  key={i}
                  className="bar"
                  style={{
                    // Height scales from 4px to 20px based on normalized level (0-1)
                    height: `${4 + v * 16}px`,
                    transition: "height 60ms ease-out, opacity 120ms ease-out",
                    opacity: Math.max(0.4, Math.min(1, 0.4 + v * 0.6)),
                  }}
                />
              ))}
            </div>
          )}
          {state === "transcribing" && (
            <div className="connecting-container">
              <div className="connecting-text">{t("overlay.transcribing")}</div>
            </div>
          )}
          {state === "processing" && (
            <div className="connecting-container">
              <div className="connecting-text">{t("overlay.processing")}</div>
            </div>
          )}
          {state === "connecting" && (
            <div className="connecting-container">
              <div className="connecting-text">
                {t("overlay.starting", "Starting microphone...")}
              </div>
            </div>
          )}
          {state === "cancelling" && (
            <div className="connecting-container">
              <div className="connecting-text">
                {t("overlay.cancelling", "Cancelling...")}
              </div>
            </div>
          )}
        </div>

        <div className="overlay-right">
          {(state === "recording" || state === "transcribing" || state === "processing" || state === "connecting" || state === "cancelling") && (
            <div
              className={`cancel-button ${state === "cancelling" ? "disabled" : ""}`}
              onClick={() => {
                if (state !== "cancelling") {
                  commands.cancelOperation();
                }
              }}
            >
              {state !== "cancelling" && <CancelIcon />}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default RecordingOverlay;
