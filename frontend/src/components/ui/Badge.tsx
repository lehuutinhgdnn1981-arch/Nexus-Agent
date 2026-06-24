import { type ReactNode } from 'react';
import clsx from 'clsx';

type Variant = 'default' | 'success' | 'warning' | 'danger' | 'info' | 'accent';

interface BadgeProps {
  variant?: Variant;
  children: ReactNode;
  className?: string;
}

const variantClasses: Record<Variant, string> = {
  default: 'bg-nexus-800 text-nexus-300 border-nexus-700',
  success: 'bg-emerald-900/40 text-success border-emerald-700',
  warning: 'bg-amber-900/40 text-warning border-amber-700',
  danger: 'bg-red-900/40 text-danger border-red-700',
  info: 'bg-blue-900/40 text-info border-blue-700',
  accent: 'bg-accent-900/40 text-accent-300 border-accent-700',
};

export function Badge({ variant = 'default', children, className }: BadgeProps) {
  return (
    <span
      className={clsx(
        'inline-flex items-center rounded-full border px-2 py-0.5 text-2xs font-medium',
        variantClasses[variant],
        className,
      )}
    >
      {children}
    </span>
  );
}
