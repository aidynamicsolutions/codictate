import { useEffect } from "react";
import { useUserProfileStore } from "../stores/userProfileStore";
import type { UserProfile } from "@/bindings";

interface UseUserProfileReturn {
  // State
  profile: UserProfile | null;
  isLoading: boolean;
  isUpdating: (key: string) => boolean;

  // Actions
  updateProfile: <K extends keyof UserProfile>(
    key: K,
    value: UserProfile[K],
  ) => Promise<void>;
  refreshProfile: () => Promise<void>;

  // Convenience getters
  getField: <K extends keyof UserProfile>(key: K) => UserProfile[K] | undefined;
}

export const useUserProfile = (): UseUserProfileReturn => {
  const store = useUserProfileStore();

  // Initialize on first mount
  useEffect(() => {
    if (store.isLoading) {
      store.initialize();
    }
  }, [store.initialize, store.isLoading]);

  return {
    profile: store.profile,
    isLoading: store.isLoading,
    isUpdating: store.isUpdatingKey,
    updateProfile: store.updateProfile,
    refreshProfile: store.refreshProfile,
    getField: store.getField,
  };
};
