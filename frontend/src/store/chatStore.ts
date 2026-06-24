import { create } from 'zustand';
import type { AgentEvent, ToolResult } from '@bindings/types';
import * as ipc from '@bindings/ipc';

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  toolCalls?: Array<{
    call_id: string;
    tool: string;
    input: Record<string, unknown>;
    result?: ToolResult;
    status: 'pending' | 'running' | 'done' | 'error';
  }>;
  isStreaming?: boolean;
  createdAt: number;
}

interface ChatState {
  messagesBySession: Record<string, ChatMessage[]>;
  activeRunId: string | null;
  isStreaming: boolean;
  error: string | null;

  send: (sessionId: string, message: string, provider?: string, model?: string) => Promise<void>;
  cancel: () => Promise<void>;
  handleEvent: (event: AgentEvent) => void;
  clearSession: (sessionId: string) => void;
  clearError: () => void;
}

// Generate a unique local ID without external deps
const localId = () =>
  `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;

export const useChatStore = create<ChatState>((set, get) => ({
  messagesBySession: {},
  activeRunId: null,
  isStreaming: false,
  error: null,

  send: async (sessionId, message, provider, model) => {
    if (get().isStreaming) {
      set({ error: 'Already streaming — cancel first' });
      return;
    }
    // Add user message immediately
    const userMsg: ChatMessage = {
      id: localId(),
      role: 'user',
      content: message,
      createdAt: Date.now(),
    };
    set((s) => ({
      messagesBySession: {
        ...s.messagesBySession,
        [sessionId]: [...(s.messagesBySession[sessionId] ?? []), userMsg],
      },
      isStreaming: true,
      error: null,
    }));

    try {
      await ipc.chatSend({ session_id: sessionId, message, provider, model });
    } catch (e) {
      set({ isStreaming: false, error: e instanceof Error ? e.message : JSON.stringify(e) });
    }
  },

  cancel: async () => {
    const runId = get().activeRunId;
    if (runId) {
      try {
        await ipc.chatCancel(runId);
      } catch (e) {
        set({ error: e instanceof Error ? e.message : JSON.stringify(e) });
      }
    }
    set({ isStreaming: false, activeRunId: null });
  },

  handleEvent: (event) => {
    switch (event.kind) {
      case 'turn_start': {
        set({ activeRunId: event.run_id, isStreaming: true, error: null });
        // Add placeholder assistant message
        const assistantMsg: ChatMessage = {
          id: localId(),
          role: 'assistant',
          content: '',
          isStreaming: true,
          toolCalls: [],
          createdAt: Date.now(),
        };
        set((s) => ({
          messagesBySession: {
            ...s.messagesBySession,
            [event.session_id]: [
              ...(s.messagesBySession[event.session_id] ?? []),
              assistantMsg,
            ],
          },
        }));
        break;
      }
      case 'delta': {
        set((s) => {
          const msgs = s.messagesBySession[event.session_id] ?? [];
          const updated = [...msgs];
          const last = updated[updated.length - 1];
          if (last && last.role === 'assistant') {
            updated[updated.length - 1] = {
              ...last,
              content: last.content + event.text,
            };
          }
          return {
            messagesBySession: { ...s.messagesBySession, [event.session_id]: updated },
          };
        });
        break;
      }
      case 'tool_call': {
        set((s) => {
          // Find session containing the active run
          const sessionId = Object.keys(s.messagesBySession).find((sid) =>
            s.messagesBySession[sid].some((m) => m.isStreaming),
          );
          if (!sessionId) return s;
          const msgs = s.messagesBySession[sessionId] ?? [];
          const updated = [...msgs];
          for (let i = updated.length - 1; i >= 0; i--) {
            if (updated[i].role === 'assistant') {
              const tc = {
                call_id: event.call_id,
                tool: event.tool,
                input: event.input,
                status: 'running' as const,
              };
              updated[i] = {
                ...updated[i],
                toolCalls: [...(updated[i].toolCalls ?? []), tc],
              };
              break;
            }
          }
          return {
            messagesBySession: { ...s.messagesBySession, [sessionId]: updated },
          };
        });
        break;
      }
      case 'tool_result': {
        set((s) => {
          // Need session_id from active run — find via activeRunId match
          // For simplicity, search all sessions
          const newMap: Record<string, ChatMessage[]> = {};
          for (const [sid, msgs] of Object.entries(s.messagesBySession)) {
            const updated = msgs.map((m) => {
              if (m.role !== 'assistant' || !m.toolCalls) return m;
              const tcIdx = m.toolCalls.findIndex((tc) => tc.call_id === event.call_id);
              if (tcIdx < 0) return m;
              const newTcs = [...m.toolCalls];
              newTcs[tcIdx] = {
                ...newTcs[tcIdx],
                result: event.result,
                status: event.result.ok ? 'done' : 'error',
              };
              return { ...m, toolCalls: newTcs };
            });
            newMap[sid] = updated;
          }
          return { messagesBySession: newMap };
        });
        break;
      }
      case 'iteration_end': {
        // Could log this somewhere; for now noop
        break;
      }
      case 'done': {
        set((s) => {
          const msgs = s.messagesBySession[event.session_id] ?? [];
          const updated = [...msgs];
          const last = updated[updated.length - 1];
          if (last && last.role === 'assistant') {
            updated[updated.length - 1] = { ...last, isStreaming: false };
          }
          return {
            messagesBySession: { ...s.messagesBySession, [event.session_id]: updated },
            isStreaming: false,
            activeRunId: null,
          };
        });
        break;
      }
      case 'error': {
        set({ isStreaming: false, activeRunId: null, error: event.message });
        break;
      }
      case 'cancelled': {
        set((s) => {
          // Mark last assistant message as not streaming
          const newMap: Record<string, ChatMessage[]> = {};
          for (const [sid, msgs] of Object.entries(s.messagesBySession)) {
            const updated = [...msgs];
            const last = updated[updated.length - 1];
            if (last && last.role === 'assistant') {
              updated[updated.length - 1] = { ...last, isStreaming: false };
            }
            newMap[sid] = updated;
          }
          return {
            messagesBySession: newMap,
            isStreaming: false,
            activeRunId: null,
          };
        });
        break;
      }
      case 'approval_request': {
        // Forwarded to approval store via hook; noop here
        break;
      }
    }
  },

  clearSession: (sessionId) =>
    set((s) => {
      const newMap = { ...s.messagesBySession };
      delete newMap[sessionId];
      return { messagesBySession: newMap };
    }),

  clearError: () => set({ error: null }),
}));
