import { useEffect } from "react";
import { useDictionaryStore } from "@/stores/dictionaryStore";
import type { CustomWordEntry } from "@/bindings";

interface UseDictionaryReturn {
  entries: CustomWordEntry[];
  isLoading: boolean;
  isSaving: boolean;
  refreshDictionary: () => Promise<void>;
  setDictionary: (entries: CustomWordEntry[]) => Promise<void>;
}

export const useDictionary = (): UseDictionaryReturn => {
  const store = useDictionaryStore();

  useEffect(() => {
    if (store.isLoading) {
      store.initialize();
    }
  }, [store.initialize, store.isLoading]);

  return {
    entries: store.entries,
    isLoading: store.isLoading,
    isSaving: store.isSaving,
    refreshDictionary: store.refreshDictionary,
    setDictionary: store.setDictionary,
  };
};
