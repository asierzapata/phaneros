import React, { createContext, useContext, useState } from 'react';
import { cn } from '@/lib/utils';

interface TabsContextValue {
  value: string;
  onValueChange: (value: string) => void;
}

const TabsContext = createContext<TabsContextValue | undefined>(undefined);

export interface TabsProps {
  defaultValue?: string;
  value?: string;
  onValueChange?: (value: string) => void;
  children?: React.ReactNode;
  className?: string;
}

export const Tabs: React.FC<TabsProps> = ({
  defaultValue = '',
  value: controlledValue,
  onValueChange,
  children,
  className,
}) => {
  const [uncontrolledValue, setUncontrolledValue] = useState(defaultValue);
  const isControlled = controlledValue !== undefined;
  const activeValue = isControlled ? controlledValue : uncontrolledValue;

  const handleValueChange = (val: string) => {
    if (!isControlled) {
      setUncontrolledValue(val);
    }
    onValueChange?.(val);
  };

  return (
    <TabsContext.Provider value={{ value: activeValue, onValueChange: handleValueChange }}>
      <div className={cn('w-full font-sans', className)}>{children}</div>
    </TabsContext.Provider>
  );
};

export const TabsList = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      role="tablist"
      className={cn(
        'inline-flex items-center justify-center p-1 bg-muted/80 backdrop-blur-xs rounded-xl border border-border text-muted-foreground shadow-inner',
        className
      )}
      {...props}
    >
      {children}
    </div>
  )
);
TabsList.displayName = 'TabsList';

export interface TabsTriggerProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  value: string;
}

export const TabsTrigger = React.forwardRef<HTMLButtonElement, TabsTriggerProps>(
  ({ className, value, children, onClick, ...props }, ref) => {
    const context = useContext(TabsContext);
    const isSelected = context?.value === value;

    const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
      context?.onValueChange(value);
      onClick?.(e);
    };

    return (
      <button
        ref={ref}
        type="button"
        role="tab"
        aria-selected={isSelected}
        onClick={handleClick}
        className={cn(
          'inline-flex items-center justify-center px-4 py-1.5 text-xs font-semibold rounded-lg transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring cursor-pointer select-none',
          isSelected
            ? 'bg-card text-foreground shadow-sm font-bold border border-border/50'
            : 'text-muted-foreground hover:text-foreground hover:bg-card/50',
          className
        )}
        {...props}
      >
        {children}
      </button>
    );
  }
);
TabsTrigger.displayName = 'TabsTrigger';

export interface TabsContentProps extends React.HTMLAttributes<HTMLDivElement> {
  value: string;
}

export const TabsContent = React.forwardRef<HTMLDivElement, TabsContentProps>(
  ({ className, value, children, ...props }, ref) => {
    const context = useContext(TabsContext);
    if (context?.value !== value) return null;

    return (
      <div
        ref={ref}
        role="tabpanel"
        className={cn('mt-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring', className)}
        {...props}
      >
        {children}
      </div>
    );
  }
);
TabsContent.displayName = 'TabsContent';
