import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  MicrophoneIcon,
  TranscriptionIcon,
  CancelIcon,
} from "../components/icons";
import "./RecordingOverlay.css";
import { commands } from "@/bindings";
import { syncLanguageFromSettings } from "@/i18n";
import { colors } from "@/theme";

type OverlayState = "recording" | "transcribing" | "processing";

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
    const setupEventListeners = async () => {
      // Listen for show-overlay event from Rust
      const unlistenShow = await listen("show-overlay", async (event) => {
        // Sync language from settings each time overlay is shown
        await syncLanguageFromSettings();
        const overlayState = event.payload as OverlayState;
        setState(overlayState);
        setIsVisible(true);
        // Reset time when showing overlay in recording state
        if (overlayState === "recording") {
          setElapsedSecs(0);
        }
      });

      // Listen for hide-overlay event from Rust
      const unlistenHide = await listen("hide-overlay", () => {
        setIsVisible(false);
        setElapsedSecs(0);
      });

      // Listen for mic-level updates
      const unlistenLevel = await listen<number[]>("mic-level", (event) => {
        const newLevels = event.payload as number[];

        // Apply smoothing to reduce jitter
        const smoothed = smoothedLevelsRef.current.map((prev, i) => {
          const target = newLevels[i] || 0;
          return prev * 0.7 + target * 0.3; // Smooth transition
        });

        smoothedLevelsRef.current = smoothed;
        setLevels(smoothed.slice(0, 16));
      });
      
      // Listen for recording time updates
      const unlistenTime = await listen<[number, number]>("recording-time", (event) => {
        const [elapsed, max] = event.payload;
        setElapsedSecs(elapsed);
        setMaxSecs(max);
      });

      // Cleanup function
      return () => {
        unlistenShow();
        unlistenHide();
        unlistenLevel();
        unlistenTime();
      };
    };

    setupEventListeners();
  }, []);

  const getIcon = () => {
    if (state === "recording") {
      return <MicrophoneIcon />;
    } else {
      return <TranscriptionIcon />;
    }
  };

  return (
    <div 
      className={`recording-overlay-wrapper ${isVisible ? "fade-in" : ""}`}
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
                    height: `${Math.min(20, 4 + Math.pow(v, 0.7) * 16)}px`,
                    transition: "height 60ms ease-out, opacity 120ms ease-out",
                    opacity: Math.max(0.2, v * 1.7),
                  }}
                />
              ))}
            </div>
          )}
          {state === "transcribing" && (
            <div className="transcribing-text">{t("overlay.transcribing")}</div>
          )}
          {state === "processing" && (
            <div className="transcribing-text">{t("overlay.processing")}</div>
          )}
        </div>

        <div className="overlay-right">
          {state === "recording" && (
            <div
              className="cancel-button"
              onClick={() => {
                commands.cancelOperation();
              }}
            >
              <CancelIcon />
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default RecordingOverlay;
