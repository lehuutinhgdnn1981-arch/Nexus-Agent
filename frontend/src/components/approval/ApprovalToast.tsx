interface ApprovalToastProps {
  count: number;
}

export function ApprovalToast({ count }: ApprovalToastProps) {
  return (
    <div className="fixed bottom-4 right-4 z-50 rounded-md border border-nexus-700 bg-nexus-900 px-4 py-2 text-xs text-nexus-300 shadow-lg animate-slide-up">
      {count} more approval{count > 1 ? 's' : ''} pending...
    </div>
  );
}
