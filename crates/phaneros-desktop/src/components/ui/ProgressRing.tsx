import React from 'react';
import { cn } from '@/lib/utils';

export interface ProgressRingProps {
  progress?: number; // 0 to 100
  value?: number; // Alias for progress
  size?: number;
  strokeWidth?: number;
  status?: 'synced' | 'syncing' | 'warning';
  showText?: boolean;
  className?: string;
}

export const ProgressRing: React.FC<ProgressRingProps> = ({
  progress,
  value,
  size = 40,
  strokeWidth = 4,
  status = 'syncing',
  showText = false,
  className = '',
}) => {
  const rawValue = progress !== undefined ? progress : value !== undefined ? value : 0;
  const clampedProgress = Math.max(0, Math.min(100, rawValue));
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (clampedProgress / 100) * circumference;

  const statusColors = {
    synced: 'text-emerald-green',
    syncing: 'text-primary',
    warning: 'text-amber-gold',
  };

  return (
    <div className="relative inline-flex items-center justify-center" data-testid="progress-ring">
      <svg
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        className={cn('transform -rotate-90', className)}
        data-testid="progress-ring-svg"
        aria-valuenow={clampedProgress}
      >
        {/* Track Circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          stroke="currentColor"
          strokeWidth={strokeWidth}
          className="text-muted/30"
          fill="transparent"
        />
        {/* Progress Circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          stroke="currentColor"
          strokeWidth={strokeWidth}
          strokeDasharray={`${circumference}, ${circumference}`}
          strokeDashoffset={strokeDashoffset}
          strokeLinecap="round"
          className={cn('transition-all duration-300 ease-in-out', statusColors[status])}
          fill="transparent"
          data-testid="progress-ring-circle"
        />
      </svg>
      {showText && (
        <span
          className="absolute text-[10px] font-mono font-bold text-foreground"
          data-testid="progress-ring-text"
        >
          {status === 'synced' ? '✓' : `${Math.round(clampedProgress)}%`}
        </span>
      )}
    </div>
  );
};
