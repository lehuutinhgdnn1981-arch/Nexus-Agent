import { useEffect } from 'react';
import { onApprovalRequest } from '@bindings/ipc';
import { useApprovalStore } from '@store/approvalStore';

export function useApprovalEvents() {
  const addApproval = useApprovalStore((s) => s.add);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onApprovalRequest((req) => {
      addApproval(req);
    })
      .then((un) => {
        unlisten = un;
      })
      .catch((e) => {
        console.error('Failed to subscribe to approval events:', e);
      });

    return () => {
      if (unlisten) unlisten();
    };
  }, [addApproval]);
}
