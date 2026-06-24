import { useEffect } from 'react';
import { Input } from '@components/ui';
import { useSessionStore } from '@store/sessionStore';

interface SessionSearchProps {
  value: string;
  onChange: (v: string) => void;
}

export function SessionSearch({ value, onChange }: SessionSearchProps) {
  const { search } = useSessionStore();

  useEffect(() => {
    const timer = setTimeout(() => {
      search(value);
    }, 200); // debounce
    return () => clearTimeout(timer);
  }, [value, search]);

  return (
    <Input
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder="Search sessions..."
      type="search"
    />
  );
}
