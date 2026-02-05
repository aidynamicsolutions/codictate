import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { logError, logInfo } from "@/utils/logging";
import { invoke } from "@tauri-apps/api/core";

// Define the shape of our store state and actions
interface UpdateStore {
  // State
  isChecking: boolean;
  updateAvailable: boolean;
  isInstalling: boolean;
  downloadProgress: number;
  showUpToDate: boolean;
  downloadedBytes: number;
  contentLength: number;
  downloadSpeed: number;
  downloadEta: number | null;
  shouldScrollToUpdates: boolean;
  isPendingRestart: boolean;
  
  // Actions
  checkForUpdates: (manual?: boolean) => Promise<void>;
  installUpdate: () => Promise<void>;
  restartApp: () => Promise<void>;
  
  // Internal Setters (can be exposed if needed, but actions are preferred)
  setChecking: (isChecking: boolean) => void;
  setShowUpToDate: (show: boolean) => void;
  setShouldScrollToUpdates: (scroll: boolean) => void;
}

export const useUpdateStore = create<UpdateStore>()(
  subscribeWithSelector((set, get) => ({
    isChecking: false,
    updateAvailable: false,
    isInstalling: false,
    downloadProgress: 0,
    showUpToDate: false,
    downloadedBytes: 0,
    contentLength: 0,
    downloadSpeed: 0,
    downloadEta: null,
    shouldScrollToUpdates: false,
    isPendingRestart: false,

    setChecking: (isChecking) => set({ isChecking }),
    setShowUpToDate: (showUpToDate) => set({ showUpToDate }),
    setShouldScrollToUpdates: (shouldScrollToUpdates) => set({ shouldScrollToUpdates }),

    restartApp: async () => {
        try {
            await invoke("set_update_menu_text", { text: "Check for Updates..." }); // Reset text before restart just in case
            await relaunch();
        } catch (error) {
            logError(`Failed to restart app: ${error}`, "fe-updater");
        }
    },

    checkForUpdates: async (manual = false) => {
      const state = get();
      if (state.isChecking) return;
      
      const MIN_DELAY = 1000; // 1 second minimum
      const startTime = Date.now();

      try {
        set({ isChecking: true });
        const update = await check();

        if (update) {
          set({ updateAvailable: true, showUpToDate: false });
        } else {
          set({ updateAvailable: false });
          if (manual) {
             set({ showUpToDate: true });
             setTimeout(() => {
                 set({ showUpToDate: false });
             }, 3000);
          }
        }
      } catch (error) {
        logError(`Failed to check for updates: ${error}`, "fe-updater");
      } finally {
        const elapsed = Date.now() - startTime;
        if (elapsed < MIN_DELAY) {
            await new Promise(resolve => setTimeout(resolve, MIN_DELAY - elapsed));
        }
        set({ isChecking: false });
      }
    },

    installUpdate: async () => {
      try {
        set({ 
            isInstalling: true, 
            downloadProgress: 0, 
            downloadedBytes: 0, 
            contentLength: 0,
            downloadSpeed: 0,
            downloadEta: null
        });
        
        // Speed calculation variables
        let lastTime = Date.now();
        let lastBytes = 0;
        const speedSamples: number[] = [];

        try {
            await invoke("set_update_menu_text", { text: "Downloading Update..." });
        } catch (e) {
             // Ignore if command missing
        }

        const update = await check();

        if (!update) {
          logInfo("No update available during install attempt", "fe-updater");
          set({ isInstalling: false });
          try {
             await invoke("set_update_menu_text", { text: "Check for Updates..." });
          } catch (e) {}
          return;
        }

        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case "Started":
              set({ 
                  downloadedBytes: 0, 
                  contentLength: event.data.contentLength ?? 0 
              });
              lastTime = Date.now();
              lastBytes = 0;
              break;
            case "Progress":
              const now = Date.now();
              const chunkLength = event.data.chunkLength;
              const currentDownloaded = get().downloadedBytes + chunkLength;
              const total = get().contentLength;
              const progress = total > 0 ? Math.round((currentDownloaded / total) * 100) : 0;
              
              // Speed calculation logic
              const timeDiff = (now - lastTime) / 1000; // seconds
              
              let speed = get().downloadSpeed;
              let eta = get().downloadEta;

              // Update stats every 500ms approx to avoid jitter
              if (timeDiff > 0.5) { 
                  const bytesDiff = currentDownloaded - lastBytes;
                  const currentSpeed = bytesDiff / timeDiff / (1024 * 1024); // MB/s
                  
                   // Simple rolling average
                   speedSamples.push(currentSpeed);
                   if (speedSamples.length > 5) speedSamples.shift();
                   speed = speedSamples.reduce((a, b) => a + b, 0) / speedSamples.length;
                   
                   // Calculate ETA
                   if (speed > 0 && total > 0) {
                       const remainingBytes = total - currentDownloaded;
                       const remainingMB = remainingBytes / (1024 * 1024);
                       eta = Math.ceil(remainingMB / speed);
                   }
                   
                   lastTime = now;
                   lastBytes = currentDownloaded;
              }

              set({ 
                  downloadedBytes: currentDownloaded,
                  downloadProgress: Math.min(progress, 100),
                  downloadSpeed: speed,
                  downloadEta: eta
              });
              break;
          }
        });

        // Download finish. Set pending restart state.
        set({ isPendingRestart: true, isInstalling: false });
        try {
            await invoke("set_update_menu_text", { text: "Restart to Update" });
        } catch (e) {}

      } catch (error) {
        logError(`Failed to install update: ${error}`, "fe-updater");
        try {
            await invoke("set_update_menu_text", { text: "Check for Updates..." }); 
        } catch (e) {}
      } finally {
        set({ 
            isInstalling: false, 
            downloadProgress: 0,
            downloadedBytes: 0,
            contentLength: 0,
            downloadSpeed: 0,
            downloadEta: null
        });
      }
    },
  }))
);
