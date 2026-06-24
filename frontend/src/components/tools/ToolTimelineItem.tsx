import { useState } from 'react';
import clsx from 'clsx';
import type { ToolActivityItem } from '@store/toolStore';
import { Badge } from '@components/ui';

interface ToolTimelineItemProps {
  item: ToolActivityItem;
}

export function ToolTimelineItem({ item }: ToolTimelineItemProps) {
  const [expanded, setExpanded] = useState(false);

  const durationMs = item.finishedAt
    ? item.finishedAt - item.startedAt
    : Date.now() - item.startedAt;

  return (
    <div className="rounded-md border border-nexus-800 bg-nexus-900/50 p-2 text-xs">
      <button
        className="flex w-full items-center justify-between text-left"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <div
            className={clsx(
              'h-2 w-2 flex-shrink-0 rounded-full',
              item.status === 'running' && 'bg-info animate-pulse-soft',
              item.status === 'done' && 'bg-success',
              item.status === 'error' && 'bg-danger',
            )}
          />
          <span className="truncate font-mono text-accent-300">{item.tool}</span>
        </div>
        <div className="flex items-center gap-2 text-2xs text-nexus-500">
          <span>{durationMs}ms</span>
          <span>{expanded ? '▼' : '▶'}</span>
        </div>
      </button>
      {expanded && (
        <div className="mt-2 space-y-2">
          <div>
            <p className="text-2xs uppercase text-nexus-500">Input</p>
            <pre className="overflow-x-auto rounded bg-nexus-950 p-2 text-2xs font-mono text-nexus-300">
              {JSON.stringify(item.input, null, 2)}
            </pre>
          </div>
          {item.result && (
            <div>
              <p className="text-2xs uppercase text-nexus-500">
                Output ·{' '}
                <Badge variant={item.result.ok ? 'success' : 'danger'}>
                  {item.result.ok ? 'ok' : 'error'}
                </Badge>
              </p>
              <pre
                className={clsx(
                  'overflow-x-auto rounded p-2 text-2xs font-mono whitespace-pre-wrap break-all',
                  item.result.ok
                    ? 'bg-nexus-950 text-nexus-300'
                    : 'bg-red-950/50 text-danger',
                )}
              >
                {item.result.output.slice(0, 1500)}
                {item.result.output.length > 1500 && '\n...[truncated]...'}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
