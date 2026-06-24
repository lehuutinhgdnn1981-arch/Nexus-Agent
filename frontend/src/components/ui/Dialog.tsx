import { type ReactNode, useEffect } from 'react';
import clsx from 'clsx';

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
}

export function Dialog({ open, onClose, title, children, footer, className }: DialogProps) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-space-950/70 backdrop-blur-sm animate-fade-in"
      onClick={onClose}
    >
      <div
        className={clsx(
          'w-full max-w-md max-h-[85vh] rounded-2xl border border-space-700/50 bg-space-900/95 shadow-2xl animate-slide-up overflow-hidden flex flex-col backdrop-blur-xl',
          className,
        )}
        onClick={(e) => e.stopPropagation()}
      >
        {title && (
          <div className="flex items-center justify-between border-b border-space-700/50 px-6 py-4 flex-shrink-0">
            <h2 className="text-base font-semibold text-space-50">{title}</h2>
            <button
              onClick={onClose}
              className="text-space-500 hover:text-space-200 transition-colors text-lg"
            >
              ✕
            </button>
          </div>
        )}
        <div className="overflow-y-auto px-6 py-5 flex-1">
          {children}
        </div>
        {footer && (
          <div className="flex justify-end gap-2 border-t border-space-700/50 px-6 py-4 flex-shrink-0">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}
