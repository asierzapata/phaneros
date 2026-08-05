import React, { useEffect } from 'react';
import { cn } from '@/lib/utils';

export interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  closeOnBackdropClick?: boolean;
  className?: string;
}

export const Modal: React.FC<ModalProps> = ({
  isOpen,
  onClose,
  title,
  children,
  closeOnBackdropClick = true,
  className,
}) => {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    if (isOpen) {
      window.addEventListener('keydown', handleKeyDown);
    }
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget && closeOnBackdropClick) {
      onClose();
    }
  };

  const handleBackdropKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      data-testid="modal-backdrop"
      onClick={handleBackdropClick}
      onKeyDown={handleBackdropKeyDown}
      tabIndex={-1}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-4 transition-opacity animate-in fade-in"
    >
      <div
        data-testid="modal-content"
        className={cn(
          'w-full max-w-md bg-card text-card-foreground border border-border rounded-xl shadow-2xl p-6 relative flex flex-col gap-4 animate-in zoom-in-95',
          className
        )}
      >
        <div className="flex items-center justify-between pb-3 border-b border-border">
          <h2 id="modal-title" className="text-lg font-serif font-bold text-foreground">
            {title}
          </h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close modal"
            className="text-muted-foreground hover:text-foreground text-sm p-1.5 rounded-lg hover:bg-muted transition-colors cursor-pointer"
          >
            ✕
          </button>
        </div>
        <div className="font-sans text-sm text-foreground">{children}</div>
      </div>
    </div>
  );
};
