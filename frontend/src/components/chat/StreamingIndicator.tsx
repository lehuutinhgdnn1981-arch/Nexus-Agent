import { Spinner } from '@components/ui';

export function StreamingIndicator() {
  return (
    <div className="flex items-center gap-2 text-xs text-nexus-400">
      <Spinner size="sm" />
      <span>Generating response...</span>
    </div>
  );
}
