import clsx from 'clsx';
import type { ChatMessage } from '@store/chatStore';
import { MarkdownRenderer } from './MarkdownRenderer';
import { ToolCallBlock } from './ToolCallBlock';
import { Spinner } from '@components/ui';

interface MessageBubbleProps {
  message: ChatMessage;
}

export function MessageBubble({ message }: MessageBubbleProps) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';

  if (isSystem) {
    return null; // don't render system messages
  }

  return (
    <div
      className={clsx(
        'flex flex-col gap-2',
        isUser ? 'items-end' : 'items-start',
      )}
    >
      <div className="flex items-center gap-2 text-2xs text-nexus-500">
        <span className="font-medium">
          {isUser ? 'You' : 'NEXUS'}
        </span>
        <span>·</span>
        <span>{new Date(message.createdAt).toLocaleTimeString()}</span>
      </div>

      <div
        className={clsx(
          'max-w-[85%] rounded-lg px-4 py-2.5',
          isUser
            ? 'bg-accent-600 text-white'
            : 'bg-nexus-800 text-nexus-100 border border-nexus-700',
        )}
      >
        {message.content ? (
          <MarkdownRenderer content={message.content} />
        ) : message.isStreaming ? (
          <div className="flex items-center gap-2 text-sm text-nexus-400">
            <Spinner size="sm" />
            <span>Thinking...</span>
          </div>
        ) : null}
      </div>

      {message.toolCalls && message.toolCalls.length > 0 && (
        <div className="w-full space-y-2">
          {message.toolCalls.map((tc) => (
            <ToolCallBlock key={tc.call_id} toolCall={tc} />
          ))}
        </div>
      )}
    </div>
  );
}
