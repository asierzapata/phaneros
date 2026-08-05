import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import React, { useState } from 'react';

// Shadcn Primitive Harnesses for testing boundaries
const ButtonPrimitive: React.FC<{
  disabled?: boolean;
  onClick?: () => void;
  children: React.ReactNode;
}> = ({ disabled, onClick, children }) => (
  <button
    disabled={disabled}
    onClick={disabled ? undefined : onClick}
    className={`btn ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
    data-testid="shadcn-btn"
  >
    {children}
  </button>
);

const ModalPrimitive: React.FC<{
  isOpen: boolean;
  onClose?: () => void;
  closeOnBackdropClick?: boolean;
  children: React.ReactNode;
}> = ({ isOpen, onClose, closeOnBackdropClick = true, children }) => {
  if (!isOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      data-testid="modal-backdrop"
      onClick={(e) => {
        if (e.target === e.currentTarget && closeOnBackdropClick && onClose) {
          onClose();
        }
      }}
      onKeyDown={(e) => {
        if (e.key === 'Escape' && onClose) {
          onClose();
        }
      }}
      tabIndex={-1}
    >
      <div data-testid="modal-content">{children}</div>
    </div>
  );
};

const ProgressRingPrimitive: React.FC<{ value: number }> = ({ value }) => {
  // Clamp value between 0 and 100 for safety
  const clamped = Math.max(0, Math.min(100, value));
  const strokeDashoffset = 100 - clamped;

  return (
    <svg data-testid="progress-ring" viewBox="0 0 36 36" aria-valuenow={clamped}>
      <path
        data-testid="progress-ring-circle"
        strokeDasharray="100, 100"
        strokeDashoffset={strokeDashoffset}
        d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
      />
      <text data-testid="progress-ring-text">{clamped}%</text>
    </svg>
  );
};

const TabsPrimitive: React.FC<{ items?: string[] }> = ({ items = [] }) => {
  const [active, setActive] = useState<string>(items[0] || '');

  if (!items || items.length === 0) {
    return <div data-testid="empty-tabs-fallback">No tabs available</div>;
  }

  return (
    <div role="tablist" data-testid="tabs-container">
      {items.map((item) => (
        <button
          key={item}
          role="tab"
          aria-selected={active === item}
          onClick={() => setActive(item)}
          data-testid={`tab-${item}`}
        >
          {item}
        </button>
      ))}
    </div>
  );
};

describe('F3_PRIM: Shadcn Component Primitives Boundary Tests', () => {
  it('F3-T2-01: should prevent onClick handler execution when Button primitive is disabled', async () => {
    const handleClick = vi.fn();
    const user = userEvent.setup();

    render(<ButtonPrimitive disabled={true} onClick={handleClick}>Submit</ButtonPrimitive>);

    const btn = screen.getByTestId('shadcn-btn');
    expect(btn).toBeDisabled();

    await user.click(btn);
    expect(handleClick).not.toHaveBeenCalled();
  });

  it('F3-T2-02: should close Modal on Escape key press boundary', () => {
    const handleClose = vi.fn();

    render(
      <ModalPrimitive isOpen={true} onClose={handleClose}>
        <p>Modal Content</p>
      </ModalPrimitive>
    );

    const backdrop = screen.getByTestId('modal-backdrop');
    fireEvent.keyDown(backdrop, { key: 'Escape', code: 'Escape' });

    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it('F3-T2-03: should respect closeOnBackdropClick property on Modal primitive', async () => {
    const handleClose = vi.fn();
    const user = userEvent.setup();

    const { rerender } = render(
      <ModalPrimitive isOpen={true} onClose={handleClose} closeOnBackdropClick={false}>
        <p>Non-closable Backdrop</p>
      </ModalPrimitive>
    );

    const backdrop = screen.getByTestId('modal-backdrop');
    await user.click(backdrop);
    expect(handleClose).not.toHaveBeenCalled();

    rerender(
      <ModalPrimitive isOpen={true} onClose={handleClose} closeOnBackdropClick={true}>
        <p>Closable Backdrop</p>
      </ModalPrimitive>
    );

    await user.click(screen.getByTestId('modal-backdrop'));
    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it('F3-T2-04: should safely clamp ProgressRing values for 0%, 100%, negative, and overflow values', () => {
    const { rerender } = render(<ProgressRingPrimitive value={0} />);
    expect(screen.getByTestId('progress-ring-text')).toHaveTextContent('0%');

    rerender(<ProgressRingPrimitive value={100} />);
    expect(screen.getByTestId('progress-ring-text')).toHaveTextContent('100%');

    rerender(<ProgressRingPrimitive value={-25} />);
    expect(screen.getByTestId('progress-ring-text')).toHaveTextContent('0%');

    rerender(<ProgressRingPrimitive value={150} />);
    expect(screen.getByTestId('progress-ring-text')).toHaveTextContent('100%');
  });

  it('F3-T2-05: should render empty state fallback when tabs list is empty', () => {
    render(<TabsPrimitive items={[]} />);
    expect(screen.getByTestId('empty-tabs-fallback')).toHaveTextContent('No tabs available');
  });
});
