import { create } from 'zustand';
import type { SchedulerJobDto, SchedulerAddInput } from '@bindings/types';
import * as ipc from '@bindings/ipc';

interface SchedulerState {
  jobs: SchedulerJobDto[];
  loading: boolean;
  error: string | null;

  load: () => Promise<void>;
  add: (input: SchedulerAddInput) => Promise<string>;
  cancel: (id: string) => Promise<void>;
  clearError: () => void;
}

export const useSchedulerStore = create<SchedulerState>((set) => ({
  jobs: [],
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const jobs = await ipc.schedulerList();
      set({ jobs, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  add: async (input) => {
    try {
      const id = await ipc.schedulerAdd(input);
      const jobs = await ipc.schedulerList();
      set({ jobs });
      return id;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  cancel: async (id) => {
    try {
      await ipc.schedulerCancel(id);
      const jobs = await ipc.schedulerList();
      set({ jobs });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
