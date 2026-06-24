import { useApprovalStore } from '@store/approvalStore';
import { ApprovalDialog } from '../approval/ApprovalDialog';
import { ApprovalToast } from './ApprovalToast';

export function ApprovalLayer() {
  const { pending } = useApprovalStore();

  return (
    <>
      {pending.length > 0 && <ApprovalDialog request={pending[0]} />}
      {pending.length > 1 && (
        <ApprovalToast count={pending.length - 1} />
      )}
    </>
  );
}
