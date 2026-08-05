import React, { useRef } from 'react';
import { cn } from '@/lib/utils';

export interface Option<T extends string> {
  value: T;
  label: string;
}

export interface SegmentedControlProps<T extends string> {
  options?: Option<T>[];
  value: T;
  onChange: (value: T) => void;
  className?: string;
}

export function SegmentedControl<T extends string>({
  options = [],
  value,
  onChange,
  className,
}: SegmentedControlProps<T>) {
  const containerRef = useRef<HTMLDivElement>(null);

  if (!options || options.length === 0) {
    return (
      <div data-testid="empty-tabs-fallback" className="text-xs text-muted-foreground p-2 font-sans">
        No tabs available
      </div>
    );
  }

  const handleKeyDown = (e: React.KeyboardEvent, currentIndex: number) => {
    if (e.key === 'ArrowRight') {
      e.preventDefault();
      const nextIndex = (currentIndex + 1) % options.length;
      onChange(options[nextIndex].value);
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      const prevIndex = (currentIndex - 1 + options.length) % options.length;
      onChange(options[prevIndex].value);
    } else if (e.key === 'Home') {
      e.preventDefault();
      onChange(options[0].value);
    } else if (e.key === 'End') {
      e.preventDefault();
      onChange(options[options.length - 1].value);
    }
  };

  return (
    <div
      ref={containerRef}
      role="tablist"
      className={cn(
        'inline-flex p-1 bg-muted/80 backdrop-blur-xs rounded-xl font-sans border border-border shadow-xs',
        className
      )}
    >
      {options.map((option, index) => {
        const isActive = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="tab"
            aria-selected={isActive}
            tabIndex={isActive ? 0 : -1}
            onClick={() => onChange(option.value)}
            onKeyDown={(e) => handleKeyDown(e, index)}
            className={cn(
              'px-3.5 py-1.5 text-xs font-semibold rounded-lg transition-all duration-200 select-none cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
              isActive
                ? 'bg-card text-foreground shadow-sm font-bold border border-border/50'
                : 'text-muted-foreground hover:text-foreground hover:bg-card/40'
            )}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
