import React from 'react';
import { cn } from '@/lib/utils';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?:
    | 'default'
    | 'emerald'
    | 'amber'
    | 'rose'
    | 'outline'
    | 'mono'
    | 'synced'
    | 'syncing'
    | 'conflict'
    | 'paused';
  children?: React.ReactNode;
}

export const Badge = React.forwardRef<HTMLSpanElement, BadgeProps>(
  ({ className, variant = 'default', children, ...props }, ref) => {
    const baseStyles =
      'inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium font-sans transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 select-none';

    const variantStyles = {
      default: 'bg-muted text-muted-foreground border-transparent',
      emerald: 'bg-emerald-bg text-emerald-green border-emerald-border',
      amber: 'bg-amber-bg text-amber-gold border-amber-border',
      rose: 'bg-red-500/10 text-red-600 dark:text-red-400 border-red-500/20',
      outline: 'border-border text-foreground bg-transparent',
      mono: 'font-mono uppercase text-[10px] tracking-wider font-bold bg-accent text-accent-foreground border-border',
      synced: 'bg-emerald-bg text-emerald-green border-emerald-border',
      syncing: 'bg-primary/10 text-primary border-primary/20 animate-pulse',
      conflict: 'bg-amber-bg text-amber-gold border-amber-border',
      paused: 'bg-muted text-muted-foreground border-border',
    };

    return (
      <span
        ref={ref}
        className={cn(baseStyles, variantStyles[variant], className)}
        {...props}
      >
        {children}
      </span>
    );
  }
);

Badge.displayName = 'Badge';
