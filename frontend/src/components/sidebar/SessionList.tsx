import { useSessionStore } from '@store/sessionStore';
import { SessionItem } from '../sidebar/SessionItem';
import { ScrollArea } from '@components/ui';
import clsx from 'clsx';

interface SessionListProps {
  className?: string;
}

export function SessionList({ className }: SessionListProps) {
  const { sessions, activeSessionId, setActive } = useSessionStore();

  if (sessions.length === 0) {
    return (
      <div className={clsx('flex items-center justify-center p-8 text-center', className)}>
        <div>
          <p className="text-sm text-nexus-400">No sessions yet</p>
          <p className="text-2xs text-nexus-500 mt-1">Click "New Session" to start</p>
        </div>
      </div>
    );
  }

  return (
    <ScrollArea className={clsx('px-2', className)}>
      <ul className="space-y-1 py-1">
        {sessions.map((session) => (
          <li key={session.id}>
            <SessionItem
              session={session}
              active={session.id === activeSessionId}
              onClick={() => setActive(session.id)}
            />
          </li>
        ))}
      </ul>
    </ScrollArea>
  );
}
