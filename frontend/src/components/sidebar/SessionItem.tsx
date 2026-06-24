import { useState } from 'react';
import clsx from 'clsx';
import { useSessionStore } from '@store/sessionStore';
import { Button, Input, Dialog } from '@components/ui';
import type { SessionDto } from '@bindings/types';

interface SessionItemProps {
  session: SessionDto;
  active: boolean;
  onClick: () => void;
}

export function SessionItem({ session, active, onClick }: SessionItemProps) {
  const { rename, remove } = useSessionStore();
  const [menuOpen, setMenuOpen] = useState(false);
  const [renameOpen, setRenameOpen] = useState(false);
  const [newTitle, setNewTitle] = useState(session.title);

  const handleRename = async () => {
    if (newTitle.trim() && newTitle !== session.title) {
      await rename(session.id, newTitle.trim());
    }
    setRenameOpen(false);
  };

  const handleDelete = async () => {
    if (confirm(`Delete session "${session.title}"?`)) {
      await remove(session.id);
    }
    setMenuOpen(false);
  };

  return (
    <>
      <div
        className={clsx(
          'group relative flex items-center rounded-md px-3 py-2 cursor-pointer transition-colors',
          active ? 'bg-accent-600/20 text-accent-100' : 'text-nexus-300 hover:bg-nexus-800',
        )}
        onClick={onClick}
      >
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{session.title}</p>
          <p className="text-2xs text-nexus-500">
            {session.provider} · {session.model}
          </p>
        </div>
        <button
          className="ml-2 opacity-0 group-hover:opacity-100 text-nexus-400 hover:text-nexus-100"
          onClick={(e) => {
            e.stopPropagation();
            setMenuOpen(!menuOpen);
          }}
        >
          ⋯
        </button>

        {menuOpen && (
          <div
            className="absolute right-2 top-8 z-10 w-32 rounded-md border border-nexus-700 bg-nexus-900 py-1 shadow-lg"
            onClick={(e) => e.stopPropagation()}
          >
            <button
              className="block w-full px-3 py-1.5 text-left text-xs text-nexus-200 hover:bg-nexus-800"
              onClick={() => {
                setRenameOpen(true);
                setMenuOpen(false);
              }}
            >
              Rename
            </button>
            <button
              className="block w-full px-3 py-1.5 text-left text-xs text-danger hover:bg-nexus-800"
              onClick={handleDelete}
            >
              Delete
            </button>
          </div>
        )}
      </div>

      <Dialog
        open={renameOpen}
        onClose={() => setRenameOpen(false)}
        title="Rename session"
        footer={
          <>
            <Button variant="ghost" size="sm" onClick={() => setRenameOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary" size="sm" onClick={handleRename}>
              Save
            </Button>
          </>
        }
      >
        <Input
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          autoFocus
          onKeyDown={(e) => e.key === 'Enter' && handleRename()}
        />
      </Dialog>
    </>
  );
}
