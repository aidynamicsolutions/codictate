
import { useEffect } from "react";
import { useUpdateStore } from "../stores/updateStore";
import { useSettings } from "./useSettings";

export const useUpdateCheck = () => {
  const store = useUpdateStore();
  const { settings, isLoading } = useSettings();
  const settingsLoaded = !isLoading && settings !== null;
  const updateChecksEnabled = settings?.update_checks_enabled ?? false;

  useEffect(() => {
      if (!settingsLoaded || !updateChecksEnabled) return;
      
      // Initial check on load if enabled
      // We check if we already checked to avoid double checking if multiple components mount this hook
      // But store doesn't track "hasCheckedOnce". 
      // For now, let's just trigger it safely (store.isChecking prevents overlap).
      store.checkForUpdates(false);
  }, [settingsLoaded, updateChecksEnabled]);

  return store;
};
