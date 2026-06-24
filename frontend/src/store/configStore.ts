import { create } from 'zustand';
import type { AppConfig } from '@bindings/types';
import * as ipc from '@bindings/ipc';

interface ConfigState {
  config: AppConfig | null;
  loading: boolean;
  error: string | null;

  load: () => Promise<void>;
  patch: (patch: Record<string, unknown>) => Promise<void>;
  clearError: () => void;
}

export const useConfigStore = create<ConfigState>((set) => ({
  config: null,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const config = await ipc.configGet();
      set({ config, loading: false });
    } catch (e) {
      set({ loading: false, error: e instanceof Error ? e.message : JSON.stringify(e) });
    }
  },

  patch: async (patch) => {
    try {
      const config = await ipc.configSet(patch);
      set({ config });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : JSON.stringify(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
