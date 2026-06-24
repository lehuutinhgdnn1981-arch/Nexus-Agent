import clsx from 'clsx';
import type { SessionDto } from '@bindings/types';

interface ChatHeaderProps {
  session?: SessionDto;
}

export function ChatHeader({ session }: ChatHeaderProps) {
  return (
    <header className="flex items-center justify-between border-b border-nexus-800 bg-nexus-900/50 px-6 py-3 backdrop-blur">
      <div className="min-w-0 flex-1">
        <h2 className="truncate text-base font-semibold text-nexus-100">
          {session?.title ?? 'Chat'}
        </h2>
        <p className="text-2xs text-nexus-400">
          {session ? `${session.provider} · ${session.model}` : ''}
        </p>
      </div>
      <div className="flex items-center gap-2">
        <span
          className={clsx(
            'inline-flex h-2 w-2 rounded-full',
            session ? 'bg-success animate-pulse-soft' : 'bg-nexus-600',
          )}
        />
        <span className="text-2xs text-nexus-400">
          {session ? 'Active' : 'No session'}
        </span>
      </div>
    </header>
  );
}
