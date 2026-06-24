import { useState, useEffect, useRef, type KeyboardEvent } from 'react';
import clsx from 'clsx';
import { paletteSearch, type PaletteItem } from '@bindings/palette';
import { useSessionStore } from '@store/sessionStore';
import { useChatStore } from '@store/chatStore';
import { Spinner, Badge } from '@components/ui';
import { formatRelativeTime } from '@lib/format';

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
}

export function CommandPalette({ open, onClose }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [items, setItems] = useState<PaletteItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const { setActive, create: createSession } = useSessionStore();
  const { send: chatSend, clearSession: clearChat } = useChatStore();
  const activeSessionId = useSessionStore((s) => s.activeSessionId);

  useEffect(() => {
    if (!open) {
      setQuery('');
      setItems([]);
      setSelectedIndex(0);
      return;
    }
    setTimeout(() => inputRef.current?.focus(), 50);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    const timer = setTimeout(async () => {
      try {
        const results = await paletteSearch(query, 5);
        setItems(results);
        setSelectedIndex(0);
      } catch (e) {
        console.error('palette search failed:', e);
        setItems([]);
      } finally {
        setLoading(false);
      }
    }, 150);
    return () => clearTimeout(timer);
  }, [query, open]);

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, items.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const item = items[selectedIndex];
      if (item) handleSelect(item);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  };

  useEffect(() => {
    const el = listRef.current?.querySelector(`[data-idx="${selectedIndex}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  const handleSelect = async (item: PaletteItem) => {
    try {
      switch (item.kind) {
        case 'quick_action': {
          const actionId = item.id;
          if (actionId === 'new_session') {
            await createSession({ title: `Session ${new Date().toLocaleString()}` });
          } else if (actionId === 'search_web' && activeSessionId) {
            await chatSend(activeSessionId, `Search the web for: ${query}`);
          } else if (actionId === 'remember' && activeSessionId) {
            await chatSend(activeSessionId, `Remember this: ${query}`);
          } else if (actionId === 'schedule' && activeSessionId) {
            await chatSend(activeSessionId, `Schedule a reminder: ${query}`);
          } else if (actionId === 'clear_chat' && activeSessionId) {
            clearChat(activeSessionId);
          }
          break;
        }
        case 'session': {
          if (item.id) setActive(item.id);
          break;
        }
        case 'memory': {
          if (item.id && activeSessionId) {
            await chatSend(activeSessionId, `Tell me about memory: ${item.content?.slice(0, 80)}`);
          }
          break;
        }
        case 'tool': {
          if (item.name && activeSessionId) {
            await chatSend(activeSessionId, `Use tool ${item.name} for: ${query}`);
          }
          break;
        }
        case 'scheduled_job':
          break;
      }
    } catch (e) {
      console.error('palette action failed:', e);
    } finally {
      onClose();
    }
  };

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/60 pt-[15vh] animate-fade-in"
      onClick={onClose}
    >
      <div
        className="w-full max-w-2xl rounded-xl border border-nexus-700 bg-nexus-900 shadow-2xl animate-slide-up"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-3 border-b border-nexus-800 px-4 py-3">
          <span className="text-nexus-400">🔍</span>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search sessions, memories, tools, or run a command..."
            className="flex-1 bg-transparent text-base text-nexus-100 placeholder:text-nexus-500 focus:outline-none"
          />
          {loading && <Spinner size="sm" />}
          <kbd className="rounded border border-nexus-700 bg-nexus-800 px-1.5 py-0.5 text-2xs text-nexus-400">
            ESC
          </kbd>
        </div>

        <div ref={listRef} className="max-h-[60vh] overflow-y-auto p-2">
          {items.length === 0 && !loading ? (
            <div className="p-8 text-center text-sm text-nexus-500">
              {query ? `No results for "${query}"` : 'Type to search...'}
            </div>
          ) : (
            <ul className="space-y-0.5">
              {items.map((item, idx) => (
                <li
                  key={`${item.kind}-${item.id ?? item.name ?? idx}`}
                  data-idx={idx}
                  onClick={() => handleSelect(item)}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  className={clsx(
                    'flex cursor-pointer items-center gap-3 rounded-md px-3 py-2.5 transition-colors',
                    idx === selectedIndex
                      ? 'bg-accent-600/20 text-accent-100'
                      : 'text-nexus-200 hover:bg-nexus-800',
                  )}
                >
                  <PaletteItemIcon item={item} />
                  <div className="min-w-0 flex-1">
                    <PaletteItemContent item={item} />
                  </div>
                  <PaletteItemMeta item={item} />
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-nexus-800 px-4 py-2 text-2xs text-nexus-500">
          <div className="flex items-center gap-3">
            <span>
              <kbd className="rounded border border-nexus-700 bg-nexus-800 px-1">↑↓</kbd> navigate
            </span>
            <span>
              <kbd className="rounded border border-nexus-700 bg-nexus-800 px-1">↵</kbd> select
            </span>
            <span>
              <kbd className="rounded border border-nexus-700 bg-nexus-800 px-1">ESC</kbd> close
            </span>
          </div>
          <span>{items.length} results</span>
        </div>
      </div>
    </div>
  );
}

function PaletteItemIcon({ item }: { item: PaletteItem }) {
  const icon = (() => {
    switch (item.kind) {
      case 'quick_action': return item.icon ?? '⚡';
      case 'session': return '💬';
      case 'memory': return '🧠';
      case 'tool': return '🔧';
      case 'scheduled_job': return '⏰';
    }
  })();
  return <span className="text-lg">{icon}</span>;
}

function PaletteItemContent({ item }: { item: PaletteItem }) {
  switch (item.kind) {
    case 'quick_action':
      return (
        <>
          <p className="text-sm font-medium">{item.title}</p>
          <p className="text-2xs text-nexus-400">{item.description}</p>
        </>
      );
    case 'session':
      return (
        <>
          <p className="truncate text-sm font-medium">{item.title}</p>
          <p className="text-2xs text-nexus-500">
            {item.provider} · {item.updated_at ? formatRelativeTime(item.updated_at) : ''}
          </p>
        </>
      );
    case 'memory':
      return (
        <>
          <p className="truncate text-sm">{item.content}</p>
          <p className="text-2xs text-nexus-500">
            {item.category} · {item.created_at ? formatRelativeTime(item.created_at) : ''}
          </p>
        </>
      );
    case 'tool':
      return (
        <>
          <p className="font-mono text-sm font-medium text-accent-300">{item.name}</p>
          <p className="truncate text-2xs text-nexus-400">{item.description}</p>
        </>
      );
    case 'scheduled_job':
      return (
        <>
          <p className="truncate text-sm">{item.message}</p>
          <p className="text-2xs text-nexus-500">{item.enabled ? 'enabled' : 'disabled'}</p>
        </>
      );
  }
}

function PaletteItemMeta({ item }: { item: PaletteItem }) {
  if (item.kind === 'tool' && item.permission) {
    const variant =
      item.permission === 'safe'
        ? 'success'
        : item.permission === 'requires_approval'
          ? 'warning'
          : 'danger';
    return <Badge variant={variant as any}>{item.permission}</Badge>;
  }
  return null;
}
