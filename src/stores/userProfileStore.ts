import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import type { UserProfile } from "@/bindings";
import { commands } from "@/bindings";

interface UserProfileStore {
  profile: UserProfile | null;
  isLoading: boolean;
  isUpdating: Record<string, boolean>;

  // Actions
  initialize: () => Promise<void>;
  updateProfile: <K extends keyof UserProfile>(
    key: K,
    value: UserProfile[K],
  ) => Promise<void>;
  refreshProfile: () => Promise<void>;
  getField: <K extends keyof UserProfile>(key: K) => UserProfile[K] | undefined;
  isUpdatingKey: (key: string) => boolean;

  // Internal setters
  setProfile: (profile: UserProfile | null) => void;
  setLoading: (loading: boolean) => void;
  setUpdating: (key: string, updating: boolean) => void;
}

export const useUserProfileStore = create<UserProfileStore>()(
  subscribeWithSelector((set, get) => ({
    profile: null,
    isLoading: true,
    isUpdating: {},

    // Internal setters
    setProfile: (profile) => set({ profile }),
    setLoading: (isLoading) => set({ isLoading }),
    setUpdating: (key, updating) =>
      set((state) => ({
        isUpdating: { ...state.isUpdating, [key]: updating },
      })),

    // Getters
    getField: (key) => get().profile?.[key],
    isUpdatingKey: (key) => get().isUpdating[key] || false,

    // Load profile from store
    refreshProfile: async () => {
      try {
        const result = await commands.getUserProfileCommand();
        if (result.status === "ok") {
          set({ profile: result.data, isLoading: false });
        } else {
          console.error("Failed to load user profile:", result.error);
          set({ isLoading: false });
        }
      } catch (error) {
        console.error("Failed to load user profile:", error);
        set({ isLoading: false });
      }
    },

    // Update a specific field
    updateProfile: async <K extends keyof UserProfile>(
      key: K,
      value: UserProfile[K],
    ) => {
      const { profile, setUpdating } = get();
      const updateKey = String(key);
      const originalValue = profile?.[key];

      setUpdating(updateKey, true);

      try {
        // Optimistic update
        set((state) => ({
          profile: state.profile ? { ...state.profile, [key]: value } : null,
        }));

        // Send to backend (values are JSON-stringified)
        const result = await commands.updateUserProfileSetting(
          updateKey,
          JSON.stringify(value),
        );

        if (result.status === "error") {
          console.error(`Failed to update profile ${updateKey}:`, result.error);
          // Rollback on error
          if (profile) {
            set({ profile: { ...profile, [key]: originalValue } });
          }
        }
      } catch (error) {
        console.error(`Failed to update profile ${updateKey}:`, error);
        // Rollback on error
        if (profile) {
          set({ profile: { ...profile, [key]: originalValue } });
        }
      } finally {
        setUpdating(updateKey, false);
      }
    },

    // Initialize
    initialize: async () => {
      await get().refreshProfile();
    },
  })),
);
