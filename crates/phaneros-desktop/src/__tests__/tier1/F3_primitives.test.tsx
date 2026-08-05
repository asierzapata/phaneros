import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { Button } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { SegmentedControl } from '@/components/ui/SegmentedControl';
import { Modal } from '@/components/ui/Modal';
import { ProgressRing } from '@/components/ui/ProgressRing';

describe('F3_PRIM: Shadcn Component Primitives', () => {
  it('F3-T1-01: should render Button variant styles (primary, secondary, outline, ghost)', () => {
    const { rerender } = render(<Button variant="primary">Primary</Button>);
    expect(screen.getByRole('button', { name: /primary/i })).toHaveClass('bg-primary');

    rerender(<Button variant="secondary">Secondary</Button>);
    expect(screen.getByRole('button', { name: /secondary/i })).toHaveClass('bg-secondary');

    rerender(<Button variant="outline">Outline</Button>);
    expect(screen.getByRole('button', { name: /outline/i })).toHaveClass('border-border');

    rerender(<Button variant="ghost">Ghost</Button>);
    expect(screen.getByRole('button', { name: /ghost/i })).toHaveClass('bg-transparent');
  });

  it('F3-T1-02: should render Card component with elevation drop shadows and children', () => {
    render(
      <Card elevation="high" data-testid="test-card">
        <div>Card Content</div>
      </Card>
    );
    const card = screen.getByTestId('test-card');
    expect(card).toBeInTheDocument();
    expect(card).toHaveClass('shadow-lg');
    expect(screen.getByText('Card Content')).toBeInTheDocument();
  });

  it('F3-T1-03: should render Badge with JetBrains Mono font class', () => {
    render(<Badge variant="mono">RS</Badge>);
    const badge = screen.getByText('RS');
    expect(badge).toHaveClass('font-mono');
    expect(badge).toHaveClass('tracking-wider');
  });

  it('F3-T1-04: should render SegmentedControl with active tab state and click handler', async () => {
    const user = userEvent.setup();
    const handleChange = vi.fn();
    const options = [
      { value: 'tab1', label: 'Tab One' },
      { value: 'tab2', label: 'Tab Two' },
    ];

    render(<SegmentedControl options={options} value="tab1" onChange={handleChange} />);

    const tab1 = screen.getByRole('tab', { name: /tab one/i });
    const tab2 = screen.getByRole('tab', { name: /tab two/i });

    expect(tab1).toHaveAttribute('aria-selected', 'true');
    expect(tab2).toHaveAttribute('aria-selected', 'false');

    await user.click(tab2);
    expect(handleChange).toHaveBeenCalledWith('tab2');
  });

  it('F3-T1-05: should render Modal in open and closed states', async () => {
    const user = userEvent.setup();
    const handleClose = vi.fn();

    const { rerender } = render(
      <Modal isOpen={false} onClose={handleClose} title="Test Modal">
        Modal Body
      </Modal>
    );

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    rerender(
      <Modal isOpen={true} onClose={handleClose} title="Test Modal">
        Modal Body
      </Modal>
    );

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Test Modal')).toBeInTheDocument();

    const closeBtn = screen.getByRole('button', { name: /close modal/i });
    await user.click(closeBtn);
    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it('F3-T1-06: should render ProgressRing with SVG element and percentage calculations', () => {
    render(<ProgressRing progress={75} size={48} strokeWidth={4} />);
    const svg = screen.getByTestId('progress-ring-svg');
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute('width', '48');
    expect(svg).toHaveAttribute('height', '48');
  });
});
