import { useState, type KeyboardEvent } from 'react';
import { useSessionStore } from '@store/sessionStore';
import { useChatStore } from '@store/chatStore';
import { Textarea, Button } from '@components/ui';

interface MessageInputProps {
  disabled?: boolean;
  isStreaming?: boolean;
  onCancel?: () => void;
}

export function MessageInput({ disabled, isStreaming, onCancel }: MessageInputProps) {
  const { activeSessionId } = useSessionStore();
  const { send } = useChatStore();
  const [text, setText] = useState('');

  const handleSubmit = () => {
    const trimmed = text.trim();
    if (!trimmed || !activeSessionId || disabled) return;
    void send(activeSessionId, trimmed);
    setText('');
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="border-t border-nexus-800 bg-nexus-900/50 px-6 py-4 backdrop-blur">
      <div className="mx-auto flex max-w-3xl items-end gap-2">
        <Textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a message... (Enter to send, Shift+Enter for new line)"
          rows={2}
          disabled={disabled}
          className="flex-1"
        />
        {isStreaming ? (
          <Button variant="danger" size="md" onClick={onCancel}>
            Stop
          </Button>
        ) : (
          <Button
            variant="primary"
            size="md"
            onClick={handleSubmit}
            disabled={disabled || !text.trim()}
          >
            Send
          </Button>
        )}
      </div>
    </div>
  );
}
