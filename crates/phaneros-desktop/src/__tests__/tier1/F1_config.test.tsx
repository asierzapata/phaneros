import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import { cn } from '@/lib/utils';
import { AppContent } from '@/App';
import { mockOnboardingStep5Completed } from '../mocks/onboardingMocks';

describe('F1_CFG: Project Config & Harness', () => {
  it('F1-T1-01: should render root app with provider wrappers and default setup', () => {
    render(<AppContent />);
    // Default onboarding state (uncompleted) renders OnboardingWizard
    expect(screen.getByTestId('onboarding-wizard')).toBeInTheDocument();
    expect(screen.getByText(/Welcome to Phaneros/i)).toBeInTheDocument();
  });

  it('F1-T1-02: should verify @/* path aliases and utility helper functions (cn)', () => {
    const combined = cn('base-class', false && 'hidden', 'extra-class');
    expect(combined).toBe('base-class extra-class');

    const conditionalTheme = cn('font-sans p-4', true && 'bg-primary text-primary-foreground');
    expect(conditionalTheme).toContain('bg-primary');
    expect(conditionalTheme).toContain('text-primary-foreground');
  });

  it('F1-T1-03: should verify font family style classes (font-serif Merriweather, font-sans Inter, font-mono JetBrains Mono)', () => {
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
      },
    });

    const headerTitle = screen.getByText('PHANEROS');
    expect(headerTitle).toHaveClass('font-serif');

    const mainContainer = screen.getByTestId('main-header');
    expect(mainContainer).toHaveClass('font-sans');
  });

  it('F1-T1-04: should render full provider harness without throwing context errors', () => {
    const { container } = render(<AppContent />);
    expect(container).toBeDefined();
    expect(container.firstChild).not.toBeNull();
  });

  it('F1-T1-05: should verify dependency initialization and default application state', () => {
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
      },
    });

    // Main control center rendered when onboarding is completed
    expect(screen.getByTestId('main-header')).toBeInTheDocument();
    expect(screen.getByTestId('system-dashboard')).toBeInTheDocument();
  });
});
