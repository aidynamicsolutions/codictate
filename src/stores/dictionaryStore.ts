import { create } from "zustand";
import type { CustomWordEntry } from "@/bindings";
import { commands } from "@/bindings";
import { logError } from "@/utils/logging";

interface DictionaryStore {
  entries: CustomWordEntry[];
  isLoading: boolean;
  isSaving: boolean;

  initialize: () => Promise<void>;
  refreshDictionary: () => Promise<void>;
  setDictionary: (entries: CustomWordEntry[]) => Promise<void>;
}

export const useDictionaryStore = create<DictionaryStore>()((set, get) => ({
  entries: [],
  isLoading: true,
  isSaving: false,

  initialize: async () => {
    if (!get().isLoading) {
      return;
    }
    await get().refreshDictionary();
  },

  refreshDictionary: async () => {
    try {
      const result = await commands.getUserDictionary();
      if (result.status === "ok") {
        set({ entries: result.data, isLoading: false });
      } else {
        set({ entries: [], isLoading: false });
        throw new Error(result.error);
      }
    } catch (error) {
      logError(`Failed to load dictionary: ${error}`, "fe-dictionary");
      set({ entries: [], isLoading: false });
    }
  },

  setDictionary: async (entries) => {
    set({ isSaving: true });
    try {
      const result = await commands.setUserDictionary(entries);
      if (result.status === "error") {
        throw new Error(result.error);
      }
      set({ entries });
    } finally {
      set({ isSaving: false });
    }
  },
}));
