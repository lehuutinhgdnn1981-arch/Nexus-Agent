import { useState } from 'react';
import { Dialog, Button, Badge } from '@components/ui';
import { useApprovalStore } from '@store/approvalStore';
import type { ApprovalRequest } from '@bindings/types';

interface ApprovalDialogProps {
  request: ApprovalRequest;
}

export function ApprovalDialog({ request }: ApprovalDialogProps) {
  const { respond } = useApprovalStore();
  const [busy, setBusy] = useState(false);

  const isDangerous = request.permission === 'dangerous';

  const handleRespond = async (decision: 'approved' | 'rejected') => {
    setBusy(true);
    try {
      await respond(request.id, decision);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open
      onClose={() => handleRespond('rejected')}
      title={
        isDangerous ? '⚠ Dangerous operation requires approval' : 'Approve tool call?'
      }
      footer={
        <>
          <Button variant="ghost" onClick={() => handleRespond('rejected')} disabled={busy}>
            Reject
          </Button>
          <Button
            variant={isDangerous ? 'danger' : 'primary'}
            onClick={() => handleRespond('approved')}
            disabled={busy}
          >
            {busy ? 'Working...' : 'Approve'}
          </Button>
        </>
      }
    >
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <span className="font-mono text-sm font-semibold text-accent-300">{request.tool}</span>
          <Badge
            variant={
              request.permission === 'dangerous'
                ? 'danger'
                : request.permission === 'requires_approval'
                  ? 'warning'
                  : 'success'
            }
          >
            {request.permission}
          </Badge>
        </div>

        <div>
          <p className="text-2xs uppercase text-nexus-500 mb-1">Input</p>
          <pre className="overflow-x-auto rounded bg-nexus-950 p-2 text-xs font-mono text-nexus-300 max-h-60">
            {JSON.stringify(request.input, null, 2)}
          </pre>
        </div>

        {isDangerous && (
          <div className="rounded-md border border-red-900/50 bg-red-900/20 px-3 py-2 text-xs text-danger">
            This operation is dangerous and may be irreversible. Review carefully before approving.
          </div>
        )}

        <p className="text-2xs text-nexus-500">
          Session: {request.session_id ?? '(global)'} · Run: {request.run_id.slice(0, 8)}
        </p>
      </div>
    </Dialog>
  );
}
