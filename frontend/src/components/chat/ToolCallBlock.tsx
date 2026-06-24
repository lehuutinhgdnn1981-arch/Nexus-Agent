import { useState } from 'react';
import clsx from 'clsx';
import { Badge } from '@components/ui';
import type { ToolResult } from '@bindings/types';

interface ToolCall {
  call_id: string;
  tool: string;
  input: Record<string, unknown>;
  result?: ToolResult;
  status: 'pending' | 'running' | 'done' | 'error';
}

interface ToolCallBlockProps {
  toolCall: ToolCall;
}

export function ToolCallBlock({ toolCall }: ToolCallBlockProps) {
  const [expanded, setExpanded] = useState(false);

  const statusBadge = () => {
    switch (toolCall.status) {
      case 'running':
        return (
          <Badge variant="info" className="animate-pulse-soft">
            running
          </Badge>
        );
      case 'done':
        return <Badge variant="success">done</Badge>;
      case 'error':
        return <Badge variant="danger">error</Badge>;
      default:
        return <Badge variant="default">pending</Badge>;
    }
  };

  return (
    <div className="rounded-md border border-nexus-700 bg-nexus-900/80 text-xs">
      <button
        className="flex w-full items-center justify-between px-3 py-2 hover:bg-nexus-800/50"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2">
          <span className="text-nexus-400">▸</span>
          <span className="font-mono font-medium text-accent-300">{toolCall.tool}</span>
          {statusBadge()}
        </div>
        <span className="text-nexus-500 text-2xs">
          {expanded ? '▼' : '▶'} details
        </span>
      </button>
      {expanded && (
        <div className="border-t border-nexus-800 px-3 py-2 space-y-2">
          <div>
            <p className="text-2xs uppercase text-nexus-500 mb-1">Input</p>
            <pre className="overflow-x-auto rounded bg-nexus-950 p-2 text-2xs font-mono text-nexus-300">
              {JSON.stringify(toolCall.input, null, 2)}
            </pre>
          </div>
          {toolCall.result && (
            <div>
              <p className="text-2xs uppercase text-nexus-500 mb-1">
                Output ({toolCall.result.ok ? 'success' : 'error'}, {toolCall.result.duration_ms}ms)
              </p>
              <pre
                className={clsx(
                  'overflow-x-auto rounded p-2 text-2xs font-mono whitespace-pre-wrap break-all',
                  toolCall.result.ok
                    ? 'bg-nexus-950 text-nexus-300'
                    : 'bg-red-950/50 text-danger',
                )}
              >
                {toolCall.result.output.slice(0, 2000)}
                {toolCall.result.output.length > 2000 && '\n...[truncated]...'}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
