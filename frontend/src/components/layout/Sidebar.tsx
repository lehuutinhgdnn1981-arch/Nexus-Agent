import { useState, useEffect } from 'react';
import { useSessionStore } from '@store/sessionStore';
import { Button, Input } from '@components/ui';
import { SessionList } from '../sidebar/SessionList';
import { NewSessionButton } from '../sidebar/NewSessionButton';
import { SessionSearch } from '../sidebar/SessionSearch';
import clsx from 'clsx';

interface SidebarProps {
  className?: string;
}

export function Sidebar({ className }: SidebarProps) {
  const { load, sessions } = useSessionStore();
  const [query, setQuery] = useState('');

  useEffect(() => {
    load();
  }, [load]);

  return (
    <aside
      className={clsx(
        'flex flex-col border-r border-nexus-800 bg-nexus-900 w-72 flex-shrink-0',
        className,
      )}
    >
      {/* Header */}
      <div className="flex items-center gap-2 border-b border-nexus-800 px-4 py-3">
        <div className="flex h-7 w-7 items-center justify-center rounded-md bg-accent-600">
          <span className="text-sm font-bold text-white">N</span>
        </div>
        <div className="flex-1">
          <h1 className="text-sm font-semibold text-nexus-100">NEXUS</h1>
          <p className="text-2xs text-nexus-400">{sessions.length} sessions</p>
        </div>
      </div>

      {/* New session button */}
      <div className="p-3">
        <NewSessionButton />
      </div>

      {/* Search */}
      <div className="px-3 pb-2">
        <SessionSearch value={query} onChange={setQuery} />
      </div>

      {/* Session list */}
      <SessionList className="flex-1" />

      {/* Footer */}
      <div className="border-t border-nexus-800 px-4 py-2 text-2xs text-nexus-500">
        NEXUS v0.1.0
      </div>
    </aside>
  );
}

// Re-export for convenience
export default Sidebar;
