// IPC wrapper quanh Tauri invoke + event listen.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
  AgentEvent,
  AppConfig,
  BrowserActionInput,
  ChatSendInput,
  ChatSendOutput,
  CreateSessionInput,
  MemoryDto,
  MemoryRecallInput,
  MemorySaveInput,
  SchedulerAddInput,
  SchedulerJobDto,
  SessionDto,
  ToolInfoDto,
  ToolInvokeInput,
  ToolResult,
  ApprovalRespondInput,
  ApprovalRequest,
} from './types';

// === Chat ===
export const chatSend = (input: ChatSendInput): Promise<ChatSendOutput> =>
  invoke<ChatSendOutput>('chat_send', { input });
export const chatCancel = (runId: string): Promise<void> =>
  invoke<void>('chat_cancel', { runId });

// === Sessions ===
export const sessionCreate = (input: CreateSessionInput): Promise<SessionDto> =>
  invoke<SessionDto>('session_create', { input });
export const sessionList = (limit?: number): Promise<SessionDto[]> =>
  invoke<SessionDto[]>('session_list', { limit });
export const sessionSearch = (query: string, limit?: number): Promise<SessionDto[]> =>
  invoke<SessionDto[]>('session_search', { query, limit });
export const sessionRename = (id: string, title: string): Promise<void> =>
  invoke<void>('session_rename', { id, title });
export const sessionDelete = (id: string): Promise<void> =>
  invoke<void>('session_delete', { id });

// === Memory ===
export const memorySave = (input: MemorySaveInput): Promise<string> =>
  invoke<string>('memory_save', { input });
export const memoryRecall = (input: MemoryRecallInput): Promise<MemoryDto[]> =>
  invoke<MemoryDto[]>('memory_recall', { input });
export const memoryList = (limit?: number): Promise<MemoryDto[]> =>
  invoke<MemoryDto[]>('memory_list', { limit });
export const memoryDelete = (id: string): Promise<void> =>
  invoke<void>('memory_delete', { id });

// === Scheduler ===
export const schedulerAdd = (input: SchedulerAddInput): Promise<string> =>
  invoke<string>('scheduler_add', { input });
export const schedulerList = (): Promise<SchedulerJobDto[]> =>
  invoke<SchedulerJobDto[]>('scheduler_list');
export const schedulerCancel = (id: string): Promise<void> =>
  invoke<void>('scheduler_cancel', { id });

// === Tools ===
export const toolList = (): Promise<ToolInfoDto[]> => invoke<ToolInfoDto[]>('tool_list');
export const toolInvoke = (input: ToolInvokeInput): Promise<ToolResult> =>
  invoke<ToolResult>('tool_invoke', { input });

// === Browser ===
export const browserAction = (action: BrowserActionInput): Promise<Record<string, unknown>> =>
  invoke<Record<string, unknown>>('browser_action', { action });
export const browserShutdown = (): Promise<void> => invoke<void>('browser_shutdown');

// === Config ===
export const configGet = (): Promise<AppConfig> => invoke<AppConfig>('config_get');
export const configSet = (patch: Record<string, unknown>): Promise<AppConfig> =>
  invoke<AppConfig>('config_set', { input: { patch } });

// === Approval ===
export const approvalRespond = (input: ApprovalRespondInput): Promise<void> =>
  invoke<void>('approval_respond', { input });
export const approvalPending = (): Promise<ApprovalRequest[]> =>
  invoke<ApprovalRequest[]>('approval_pending');

// === Custom providers ===
export const customProviderAdd = (config: import('./types').CustomProviderConfig): Promise<void> =>
  invoke<void>('custom_provider_add', { input: { config } });
export const customProviderRemove = (id: string): Promise<void> =>
  invoke<void>('custom_provider_remove', { id });
export const customProviderList = (): Promise<import('./types').CustomProviderDto[]> =>
  invoke<import('./types').CustomProviderDto[]>('custom_provider_list');
export const providerListAll = (): Promise<string[]> =>
  invoke<string[]>('provider_list_all');
export const customProviderPreset = (
  presetId: string,
  apiKey?: string,
): Promise<import('./types').CustomProviderConfig> =>
  invoke<import('./types').CustomProviderConfig>('custom_provider_preset', {
    presetId,
    apiKey: apiKey ?? null,
  });

// === Event listeners ===
export const onAgentEvent = (
  handler: (event: AgentEvent) => void,
): Promise<UnlistenFn> => {
  // Lắng nghe tất cả agent events
  const unlistens: UnlistenFn[] = [];
  const eventNames = [
    'agent:turn_start',
    'agent:delta',
    'agent:tool_call',
    'agent:tool_result',
    'agent:iteration_end',
    'agent:done',
    'agent:error',
    'agent:cancelled',
  ];
  // Combine: chỉ support 1 unlisten cho caller nên ta trả wrapper
  return new Promise((resolve, reject) => {
    Promise.all(
      eventNames.map((name) =>
        listen<AgentEvent>(name, (e) => {
          handler(e.payload);
        }),
      ),
    )
      .then((fns) => {
        fns.forEach((fn) => unlistens.push(fn));
        resolve(() => unlistens.forEach((fn) => fn()));
      })
      .catch(reject);
  });
};

export const onApprovalRequest = (
  handler: (req: ApprovalRequest) => void,
): Promise<UnlistenFn> =>
  listen<ApprovalRequest>('approval:request', (e) => handler(e.payload));

export const onSchedulerFired = (
  handler: (payload: { task_id: string; message: string }) => void,
): Promise<UnlistenFn> =>
  listen<{ task_id: string; message: string }>('scheduler:fired', (e) => handler(e.payload));
