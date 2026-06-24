import { useEffect, useRef, useState } from 'react';
import { useSessionStore } from '@store/sessionStore';
import { useChatStore } from '@store/chatStore';
import { ChatHeader } from '../chat/ChatHeader';
import { MessageList } from '../chat/MessageList';
import { MessageInput } from '../chat/MessageInput';
import { Spinner } from '@components/ui';
import clsx from 'clsx';

interface ChatPanelProps {
  className?: string;
}

export function ChatPanel({ className }: ChatPanelProps) {
  const { activeSessionId, sessions } = useSessionStore();
  const { isStreaming, cancel, error } = useChatStore();
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const activeSession = sessions.find((s) => s.id === activeSessionId);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [activeSessionId]);

  if (!activeSessionId) {
    return (
      <div
        className={clsx(
          'flex flex-1 items-center justify-center bg-nexus-950 text-nexus-500',
          className,
        )}
      >
        <div className="text-center">
          <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-2xl bg-accent-600">
            <span className="text-3xl font-bold text-white">N</span>
          </div>
          <h2 className="text-xl font-semibold text-nexus-200">Welcome to NEXUS</h2>
          <p className="mt-2 text-sm">Select a session or create a new one to start chatting.</p>
        </div>
      </div>
    );
  }

  return (
    <main className={clsx('flex flex-1 flex-col bg-nexus-950', className)}>
      <ChatHeader session={activeSession} />
      <MessageList sessionId={activeSessionId} messagesEndRef={messagesEndRef} />
      {error && (
        <div className="border-t border-red-900/40 bg-red-900/20 px-4 py-2 text-xs text-danger">
          {error}
        </div>
      )}
      <MessageInput
        disabled={!activeSessionId}
        isStreaming={isStreaming}
        onCancel={cancel}
      />
      {isStreaming && (
        <div className="absolute right-4 bottom-24 flex items-center gap-2 rounded-full border border-nexus-700 bg-nexus-900/80 px-3 py-1.5 text-2xs text-nexus-300 backdrop-blur">
          <Spinner size="sm" />
          <span>Streaming...</span>
          <button className="text-danger hover:underline" onClick={cancel}>
            Stop
          </button>
        </div>
      )}
    </main>
  );
}
