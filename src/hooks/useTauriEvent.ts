import { useEffect, useRef } from "react";
import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";
import { logError, logInfo } from "@/utils/logging";

/**
 * Custom hook for Tauri event listeners with automatic cleanup.
 *
 * This hook handles the async nature of Tauri's `listen` function and ensures
 * proper cleanup when the component unmounts, preventing memory leaks.
 *
 * @param event - The Tauri event name to listen for
 * @param handler - Callback function to handle the event
 *
 * @example
 * ```tsx
 * useTauriEvent("check-for-updates", () => {
 *   logInfo("Update check requested", "Updater");
 * });
 *
 * useTauriEvent<{ previous: string; current: string }>("audio-device-changed", (event) => {
 *   logInfo(`Device changed: ${JSON.stringify(event.payload)}`, "Audio");
 * });
 * ```
 */
export function useTauriEvent<T = unknown>(
  event: string,
  handler: EventCallback<T>
): void {
  // Use ref to always have access to latest handler without re-subscribing
  const handlerRef = useRef<EventCallback<T>>(handler);

  // Update handler ref on every render - this is safe and fast
  useEffect(() => {
    handlerRef.current = handler;
  });

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let isMounted = true;

    // Wrap handler to use ref (avoids stale closure issues)
    const wrappedHandler: EventCallback<T> = (event) => {
      if (handlerRef.current) {
        handlerRef.current(event);
      }
    };

    const setupListener = async () => {
      try {
        const unlistenFn = await listen<T>(event, wrappedHandler);
        if (isMounted) {
          unlisten = unlistenFn;
        } else {
          // Component unmounted before listen resolved - cleanup immediately
          unlistenFn();
        }
      } catch (error) {
        logError(`Failed to setup listener for event "${event}": ${error}`, "useTauriEvent");
      }
    };

    setupListener();

    return () => {
      isMounted = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, [event]);
}
