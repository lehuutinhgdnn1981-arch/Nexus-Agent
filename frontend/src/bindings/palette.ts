// Command Palette — Cmd+K / Ctrl+K shortcut mở palette search mọi thứ.

import { invoke } from '@tauri-apps/api/core';

export interface PaletteItem {
  kind: 'session' | 'memory' | 'tool' | 'scheduled_job' | 'quick_action';
  // Common fields (optional per variant)
  id?: string;
  title?: string;
  description?: string;
  icon?: string;
  score: number;

  // Session
  provider?: string;
  updated_at?: number;

  // Memory
  content?: string;
  category?: string;
  created_at?: number;

  // Tool
  name?: string;
  permission?: string;

  // ScheduledJob
  message?: string;
  enabled?: boolean;
}

export const paletteSearch = (query: string, limit = 5): Promise<PaletteItem[]> =>
  invoke<PaletteItem[]>('palette_search', { input: { query, limit } });
