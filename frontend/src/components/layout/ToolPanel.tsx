import { useToolStore, type ToolActivityItem } from '@store/toolStore';
import { ToolTimelineItem } from '../tools/ToolTimelineItem';
import { ScrollArea, Badge } from '@components/ui';
import clsx from 'clsx';

interface ToolPanelProps {
  className?: string;
}

export function ToolPanel({ className }: ToolPanelProps) {
  const { items, clear } = useToolStore();

  const running = items.filter((i) => i.status === 'running').length;
  const completed = items.filter((i) => i.status === 'done').length;
  const errored = items.filter((i) => i.status === 'error').length;

  return (
    <aside
      className={clsx(
        'flex flex-col border-l border-nexus-800 bg-nexus-900 w-80 flex-shrink-0',
        className,
      )}
    >
      {/* Header */}
      <div className="border-b border-nexus-800 px-4 py-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-nexus-100">Tool Activity</h2>
          {items.length > 0 && (
            <button
              className="text-2xs text-nexus-500 hover:text-nexus-300"
              onClick={clear}
            >
              Clear
            </button>
          )}
        </div>
        <div className="mt-2 flex items-center gap-2">
          {running > 0 && <Badge variant="info">{running} running</Badge>}
          {completed > 0 && <Badge variant="success">{completed} done</Badge>}
          {errored > 0 && <Badge variant="danger">{errored} errors</Badge>}
          {items.length === 0 && (
            <span className="text-2xs text-nexus-500">No activity yet</span>
          )}
        </div>
      </div>

      {/* Timeline */}
      <ScrollArea className="flex-1 px-2 py-2">
        {items.length === 0 ? (
          <div className="flex h-full items-center justify-center p-6 text-center">
            <div>
              <div className="mx-auto mb-2 flex h-10 w-10 items-center justify-center rounded-full bg-nexus-800 text-nexus-500">
                ◇
              </div>
              <p className="text-xs text-nexus-500">
                Tool calls from the agent will appear here in real time.
              </p>
            </div>
          </div>
        ) : (
          <ul className="space-y-2">
            {items
              .slice()
              .reverse()
              .map((item: ToolActivityItem) => (
                <li key={item.id}>
                  <ToolTimelineItem item={item} />
                </li>
              ))}
          </ul>
        )}
      </ScrollArea>
    </aside>
  );
}
