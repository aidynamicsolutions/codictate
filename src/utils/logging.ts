/**
 * Unified logging utility for Handy frontend.
 * Routes logs through Tauri to appear in the unified log file
 * with session correlation.
 */

import { commands } from "@/bindings";
import { listen } from "@tauri-apps/api/event";

let currentSessionId: string | null = null;

/**
 * Initialize the logging system by listening for session-started events.
 * Call this once at app startup.
 */
export function initLogging(): () => void {
  const unlisten = listen<string>("session-started", (event) => {
    currentSessionId = event.payload;
    log("info", `Session started: ${currentSessionId}`);
  });

  // Return cleanup function
  return () => {
    unlisten.then((fn) => fn());
  };
}

/**
 * Get the current session ID.
 */
export function getSessionId(): string | null {
  return currentSessionId;
}

/**
 * Clear the session ID (call when recording ends).
 */
export function clearSessionId(): void {
  currentSessionId = null;
}

/**
 * Log a message to the unified log file via Tauri backend.
 * Falls back to console if invoke fails.
 *
 * @param level - Log level: "error" | "warn" | "info" | "debug" | "trace"
 * @param message - Message to log
 * @param target - Optional target/component name (default: "frontend")
 */
export async function log(
  level: "error" | "warn" | "info" | "debug" | "trace",
  message: string,
  target: string = "fe"
): Promise<void> {
  try {
    await commands.logFromFrontend(
      level,
      currentSessionId,
      target,
      message,
    );
  } catch {
    // Fallback to console if invoke fails
    const sessionPrefix = currentSessionId
      ? `session=${currentSessionId} `
      : "";
    console[level === "trace" ? "debug" : level](
      `${sessionPrefix}target=${target} ${message}`
    );
  }
}

// Convenience wrappers
export const logError = (msg: string, target?: string) =>
  log("error", msg, target);
export const logWarn = (msg: string, target?: string) =>
  log("warn", msg, target);
export const logInfo = (msg: string, target?: string) =>
  log("info", msg, target);
export const logDebug = (msg: string, target?: string) =>
  log("debug", msg, target);
export const logTrace = (msg: string, target?: string) =>
  log("trace", msg, target);
