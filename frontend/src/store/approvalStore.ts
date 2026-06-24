import { create } from 'zustand';
import type { ApprovalRequest } from '@bindings/types';
import * as ipc from '@bindings/ipc';

interface ApprovalState {
  pending: ApprovalRequest[];
  add: (req: ApprovalRequest) => void;
  respond: (requestId: string, decision: 'approved' | 'rejected') => Promise<void>;
  remove: (requestId: string) => void;
  clear: () => void;
}

export const useApprovalStore = create<ApprovalState>((set, get) => ({
  pending: [],
  add: (req) =>
    set((s) => ({
      pending: s.pending.some((p) => p.id === req.id) ? s.pending : [...s.pending, req],
    })),
  respond: async (requestId, decision) => {
    try {
      await ipc.approvalRespond({ request_id: requestId, decision });
    } catch (e) {
      console.error('approval respond failed:', e);
    }
    get().remove(requestId);
  },
  remove: (requestId) =>
    set((s) => ({ pending: s.pending.filter((p) => p.id !== requestId) })),
  clear: () => set({ pending: [] }),
}));
