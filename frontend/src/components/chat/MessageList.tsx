import { useEffect, type RefObject } from 'react';
import { useChatStore, type ChatMessage } from '@store/chatStore';
import { MessageBubble } from './MessageBubble';
import { ScrollArea } from '@components/ui';

interface MessageListProps {
  sessionId: string;
  messagesEndRef: RefObject<HTMLDivElement>;
}

export function MessageList({ sessionId, messagesEndRef }: MessageListProps) {
  const { messagesBySession } = useChatStore();
  const messages = messagesBySession[sessionId] ?? [];

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, messagesEndRef]);

  if (messages.length === 0) {
    return (
      <ScrollArea className="flex-1 px-6 py-8">
        <div className="mx-auto max-w-3xl text-center text-sm text-nexus-500">
          <p>No messages yet. Start by sending a message below.</p>
        </div>
      </ScrollArea>
    );
  }

  return (
    <ScrollArea className="flex-1 px-6 py-4">
      <div className="mx-auto max-w-3xl space-y-6">
        {messages.map((msg: ChatMessage) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}
        <div ref={messagesEndRef} />
      </div>
    </ScrollArea>
  );
}
