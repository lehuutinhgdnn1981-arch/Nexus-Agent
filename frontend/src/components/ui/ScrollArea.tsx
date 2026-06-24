import { type ReactNode } from 'react';
import clsx from 'clsx';

interface ScrollAreaProps {
  children: ReactNode;
  className?: string;
}

export function ScrollArea({ children, className }: ScrollAreaProps) {
  return (
    <div
      className={clsx(
        'overflow-y-auto scrollbar-thin scrollbar-thumb-nexus-700 scrollbar-track-transparent',
        className,
      )}
    >
      {children}
    </div>
  );
}
