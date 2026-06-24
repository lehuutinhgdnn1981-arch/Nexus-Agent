import { create } from 'zustand';
import type { SessionDto, CreateSessionInput } from '@bindings/types';
import * as ipc from '@bindings/ipc';

interface SessionState {
  sessions: SessionDto[];
  activeSessionId: string | null;
  loading: boolean;
  error: string | null;

  load: () => Promise<void>;
  search: (query: string) => Promise<void>;
  create: (input?: CreateSessionInput) => Promise<SessionDto>;
  setActive: (id: string) => void;
  rename: (id: string, title: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  clearError: () => void;
}

export const useSessionStore = create<SessionState>((set, get) => ({
  sessions: [],
  activeSessionId: null,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      const sessions = await ipc.sessionList(100);
      set({ sessions, loading: false });
      if (!get().activeSessionId && sessions.length > 0) {
        set({ activeSessionId: sessions[0].id });
      }
    } catch (e) {
      set({ loading: false, error: e instanceof Error ? e.message : JSON.stringify(e) });
    }
  },

  search: async (query) => {
    set({ loading: true, error: null });
    try {
      const sessions = query.trim()
        ? await ipc.sessionSearch(query, 50)
        : await ipc.sessionList(100);
      set({ sessions, loading: false });
    } catch (e) {
      set({ loading: false, error: e instanceof Error ? e.message : JSON.stringify(e) });
    }
  },

  create: async (input) => {
    set({ loading: true, error: null });
    try {
      const session = await ipc.sessionCreate(input ?? {});
      set((s) => ({
        sessions: [session, ...s.sessions],
        activeSessionId: session.id,
        loading: false,
      }));
      return session;
    } catch (e) {
      set({ loading: false, error: e instanceof Error ? e.message : JSON.stringify(e) });
      throw e;
    }
  },

  setActive: (id) => set({ activeSessionId: id }),

  rename: async (id, title) => {
    try {
      await ipc.sessionRename(id, title);
      set((s) => ({
        sessions: s.sessions.map((sess) =>
          sess.id === id ? { ...sess, title, updated_at: Date.now() / 1000 } : sess,
        ),
      }));
    } catch (e) {
      set({ error: e instanceof Error ? e.message : JSON.stringify(e) });
    }
  },

  remove: async (id) => {
    try {
      await ipc.sessionDelete(id);
      set((s) => {
        const sessions = s.sessions.filter((sess) => sess.id !== id);
        const activeSessionId =
          s.activeSessionId === id ? sessions[0]?.id ?? null : s.activeSessionId;
        return { sessions, activeSessionId };
      });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : JSON.stringify(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
