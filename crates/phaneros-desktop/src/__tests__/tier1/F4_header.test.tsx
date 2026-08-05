import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { Header } from '@/components/main/Header';
import { AppContent } from '@/App';
import { mockOnboardingStep5Completed } from '../mocks/onboardingMocks';

describe('F4_HDR: Main Control Center Header & HIG Tabs', () => {
  it('F4-T1-01: should render header top bar with transparent background style', () => {
    render(<Header />);
    const header = screen.getByTestId('main-header');
    expect(header).toBeInTheDocument();
    expect(header).toHaveClass('bg-transparent');
  });

  it('F4-T1-02: should render fixed 180px side blocks for layout symmetry', () => {
    render(<Header />);
    const leftBlock = screen.getByTestId('header-left-block');
    const rightBlock = screen.getByTestId('header-right-block');

    expect(leftBlock).toHaveClass('w-[180px]');
    expect(rightBlock).toHaveClass('w-[180px]');
  });

  it('F4-T1-03: should render brand wordmark PHANEROS in Merriweather font', () => {
    render(<Header />);
    const wordmark = screen.getByText('PHANEROS');
    expect(wordmark).toBeInTheDocument();
    expect(wordmark).toHaveClass('font-serif');
  });

  it('F4-T1-04: should render 5 Apple HIG segmented tabs (Dashboard, Drives & Files, Conflicts, Activity, Settings)', () => {
    render(<Header />);
    expect(screen.getByRole('tab', { name: /dashboard/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /drives & files/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /conflicts/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /activity/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /settings/i })).toBeInTheDocument();
  });

  it('F4-T1-05: should switch active view when clicking on HIG header tabs', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
      },
    });

    expect(screen.getByTestId('system-dashboard')).toBeInTheDocument();

    const drivesTab = screen.getByRole('tab', { name: /drives & files/i });
    await user.click(drivesTab);
    expect(screen.getByTestId('drives-files-workspace')).toBeInTheDocument();

    const conflictsTab = screen.getByRole('tab', { name: /conflicts/i });
    await user.click(conflictsTab);
    expect(screen.getByTestId('conflicts-workspace')).toBeInTheDocument();
  });
});
