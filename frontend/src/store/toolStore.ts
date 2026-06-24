import { create } from 'zustand';
import type { ToolResult } from '@bindings/types';

export interface ToolActivityItem {
  id: string;
  run_id: string;
  call_id: string;
  tool: string;
  input: Record<string, unknown>;
  result?: ToolResult;
  status: 'running' | 'done' | 'error';
  startedAt: number;
  finishedAt?: number;
}

interface ToolState {
  items: ToolActivityItem[];
  add: (item: ToolActivityItem) => void;
  update: (callId: string, result: ToolResult) => void;
  clear: () => void;
  clearRun: (runId: string) => void;
}

export const useToolStore = create<ToolState>((set) => ({
  items: [],
  add: (item) =>
    set((s) => ({
      items: [...s.items.slice(-99), item], // keep last 100
    })),
  update: (callId, result) =>
    set((s) => ({
      items: s.items.map((it) =>
        it.call_id === callId
          ? {
              ...it,
              result,
              status: result.ok ? 'done' : 'error',
              finishedAt: Date.now(),
            }
          : it,
      ),
    })),
  clear: () => set({ items: [] }),
  clearRun: (runId) => set((s) => ({ items: s.items.filter((it) => it.run_id !== runId) })),
}));
