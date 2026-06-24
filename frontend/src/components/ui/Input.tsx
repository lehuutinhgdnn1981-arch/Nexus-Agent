import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from 'react';
import clsx from 'clsx';

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={clsx(
        'h-9 w-full rounded-md border border-nexus-700 bg-nexus-900 px-3 text-sm text-nexus-100',
        'placeholder:text-nexus-500',
        'focus:outline-none focus:ring-2 focus:ring-accent-500 focus:border-transparent',
        'disabled:opacity-50',
        className,
      )}
      {...props}
    />
  ),
);
Input.displayName = 'Input';

export const Textarea = forwardRef<
  HTMLTextAreaElement,
  TextareaHTMLAttributes<HTMLTextAreaElement>
>(({ className, ...props }, ref) => (
  <textarea
    ref={ref}
    className={clsx(
      'w-full rounded-md border border-nexus-700 bg-nexus-900 px-3 py-2 text-sm text-nexus-100',
      'placeholder:text-nexus-500 resize-none',
      'focus:outline-none focus:ring-2 focus:ring-accent-500 focus:border-transparent',
      'disabled:opacity-50',
      className,
    )}
    {...props}
  />
));
Textarea.displayName = 'Textarea';
