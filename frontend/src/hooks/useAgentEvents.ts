import { useEffect } from 'react';
import { onAgentEvent } from '@bindings/ipc';
import { useChatStore } from '@store/chatStore';
import { useToolStore } from '@store/toolStore';

/**
 * Subscribe to all agent IPC events and forward to Zustand stores.
 * Should be mounted once at app root.
 */
export function useAgentEvents() {
  const handleChatEvent = useChatStore((s) => s.handleEvent);
  const addToolItem = useToolStore((s) => s.add);
  const updateToolItem = useToolStore((s) => s.update);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    onAgentEvent((event) => {
      // Forward to chat store
      handleChatEvent(event);

      // Forward tool events to tool store
      if (event.kind === 'tool_call') {
        addToolItem({
          id: `${event.run_id}-${event.call_id}`,
          run_id: event.run_id,
          call_id: event.call_id,
          tool: event.tool,
          input: event.input,
          status: 'running',
          startedAt: Date.now(),
        });
      } else if (event.kind === 'tool_result') {
        updateToolItem(event.call_id, event.result);
      }
    })
      .then((un) => {
        unlisten = un;
      })
      .catch((e) => {
        console.error('Failed to subscribe to agent events:', e);
      });

    return () => {
      if (unlisten) unlisten();
    };
  }, [handleChatEvent, addToolItem, updateToolItem]);
}
