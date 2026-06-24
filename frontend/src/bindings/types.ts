// Auto-generated TypeScript bindings would normally come from ts_rs.
// For now we maintain them manually matching the Rust structs.

// === Permission levels ===
export type PermissionLevel = 'safe' | 'requires_approval' | 'dangerous';

// === Chat ===
export interface ChatSendInput {
  session_id: string;
  message: string;
  provider?: string;
  model?: string;
  /** Enable V2 brain (memory tiering + compression + reflection + episode memory). Default: true. */
  enable_brain?: boolean;
}

export interface ChatSendOutput {
  run_id: string;
}

// === Session ===
export interface CreateSessionInput {
  title?: string;
  provider?: string;
  model?: string;
  system_prompt?: string;
}

export interface SessionDto {
  id: string;
  title: string;
  provider: string;
  model: string;
  system_prompt: string | null;
  created_at: number;
  updated_at: number;
}

// === Memory ===
export interface MemorySaveInput {
  content: string;
  category: 'fact' | 'preference' | 'task' | 'note';
  tags?: string[];
  session_id?: string;
}

export interface MemoryRecallInput {
  query: string;
  top_k?: number;
  category?: string;
}

export interface MemoryDto {
  id: string;
  content: string;
  category: string;
  tags: string[];
  session_id: string | null;
  created_at: number;
  last_used_at: number;
  use_count: number;
}

// === Scheduler ===
export interface SchedulerAddInput {
  schedule: string;
  message: string;
  session_id?: string;
}

export interface SchedulerJobDto {
  id: string;
  kind: string;
  message: string;
  session_id: string | null;
  enabled: boolean;
  created_at: number;
}

// === Tool ===
export interface ToolInfoDto {
  name: string;
  description: string;
  permission: PermissionLevel;
  schema: Record<string, unknown>;
}

export interface ToolInvokeInput {
  name: string;
  input: Record<string, unknown>;
  session_id?: string;
}

export interface ToolResult {
  call_id: string;
  tool: string;
  ok: boolean;
  output: string;
  data?: Record<string, unknown>;
  duration_ms: number;
}

// === Browser ===
export type BrowserActionInput =
  | { kind: 'navigate'; url: string }
  | { kind: 'click'; selector: string }
  | { kind: 'type'; selector: string; text: string }
  | { kind: 'wait'; selector: string }
  | { kind: 'extract_text' }
  | { kind: 'screenshot'; full_page?: boolean };

// === Config ===
export interface AppConfig {
  agent: {
    max_iterations: number;
    max_tool_calls: number;
    default_provider: string;
    default_model: string;
    system_prompt: string | null;
  };
  llm: {
    openai: ProviderConfig;
    openrouter: ProviderConfig;
    anthropic: ProviderConfig;
    ollama: ProviderConfig;
  };
  memory: {
    embedding_provider: string;
    embedding_model: string;
    embedding_dim: number;
    recall_top_k: number;
    dedup_threshold: number;
  };
  security: {
    approval_timeout_secs: number;
    shell_timeout_secs: number;
    shell_max_output_kb: number;
  };
  browser: {
    headless: boolean;
    port: number;
  };
  search: {
    default: string;
    brave_api_key: string | null;
  };
}

export interface ProviderConfig {
  api_key: string | null;
  base_url: string | null;
  default_model: string | null;
}

// === Approval ===
export interface ApprovalRespondInput {
  request_id: string;
  decision: 'approved' | 'rejected';
}

export interface ApprovalRequest {
  id: string;
  tool: string;
  input: Record<string, unknown>;
  permission: PermissionLevel;
  session_id: string | null;
  run_id: string;
}

// === Agent events ===
export interface Usage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export interface ToolCall {
  id: string;
  type: string;
  function: { name: string; arguments: string };
}

export type AgentEvent =
  | { kind: 'turn_start'; run_id: string; session_id: string; user_message: string }
  | { kind: 'delta'; run_id: string; session_id: string; text: string }
  | {
      kind: 'tool_call';
      run_id: string;
      call_id: string;
      tool: string;
      input: Record<string, unknown>;
    }
  | { kind: 'tool_result'; run_id: string; call_id: string; result: ToolResult }
  | { kind: 'iteration_end'; run_id: string; iteration: number; tool_calls_made: number }
  | {
      kind: 'done';
      run_id: string;
      session_id: string;
      final_message: string;
      usage: Usage;
    }
  | { kind: 'error'; run_id: string; message: string }
  | { kind: 'cancelled'; run_id: string }
  | {
      kind: 'approval_request';
      run_id: string;
      request_id: string;
      tool: string;
      input: Record<string, unknown>;
      permission: string;
    };

// === IPC error ===
export interface IpcError {
  code: string;
  message: string;
}

// === Custom providers ===
export interface CustomProviderConfig {
  id: string;
  api_key: string | null;
  base_url: string;
  default_model: string | null;
  embedding_model: string | null;
  extra_headers: Record<string, string> | null;
  display_name: string | null;
  supports_tools: boolean;
  timeout_secs: number;
}

export interface CustomProviderDto {
  id: string;
  base_url: string;
  default_model: string | null;
  display_name: string | null;
  supports_tools: boolean;
  has_api_key: boolean;
}
