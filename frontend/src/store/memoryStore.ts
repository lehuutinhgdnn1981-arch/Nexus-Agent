import { create } from 'zustand';
import type { MemoryDto, MemorySaveInput } from '@bindings/types';
import * as ipc from '@bindings/ipc';

interface MemoryState {
  memories: MemoryDto[];
  loading: boolean;
  error: string | null;

  load: () => Promise<void>;
  save: (input: MemorySaveInput) => Promise<string>;
  recall: (query: string, topK?: number) => Promise<MemoryDto[]>;
  remove: (id: string) => Promise<void>;
  clearError: () => void;
}

export const useMemoryStore = create<MemoryState>((set) => ({
  memories: [],
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const memories = await ipc.memoryList(100);
      set({ memories, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  save: async (input) => {
    try {
      const id = await ipc.memorySave(input);
      // Refresh list
      const memories = await ipc.memoryList(100);
      set({ memories });
      return id;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  recall: async (query, topK = 5) => {
    try {
      return await ipc.memoryRecall({ query, top_k: topK });
    } catch (e) {
      set({ error: String(e) });
      return [];
    }
  },

  remove: async (id) => {
    try {
      await ipc.memoryDelete(id);
      set((s) => ({ memories: s.memories.filter((m) => m.id !== id) }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
