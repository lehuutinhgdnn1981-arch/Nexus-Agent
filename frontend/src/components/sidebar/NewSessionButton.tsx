import { useSessionStore } from '@store/sessionStore';
import { Button } from '@components/ui';

export function NewSessionButton() {
  const { create, loading } = useSessionStore();

  const handleCreate = async () => {
    try {
      await create({ title: `New Session ${new Date().toLocaleString()}` });
    } catch (e) {
      console.error('Failed to create session:', e);
    }
  };

  return (
    <Button variant="primary" size="md" className="w-full" onClick={handleCreate} disabled={loading}>
      + New Session
    </Button>
  );
}
