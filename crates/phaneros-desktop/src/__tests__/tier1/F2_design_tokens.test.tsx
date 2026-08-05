import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { useTheme } from '@/context/ThemeContext';
import { AppContent } from '@/App';
import { mockOnboardingStep5Completed } from '../mocks/onboardingMocks';

const ThemeTester = () => {
  const { theme, toggleTheme } = useTheme();
  return (
    <div className={theme} data-testid="theme-container">
      <span data-testid="current-theme">{theme}</span>
      <button type="button" onClick={toggleTheme}>
        Toggle Theme
      </button>
    </div>
  );
};

describe('F2_DS: OKLCH Tokens & Paper Substrate & Themes', () => {
  it('F2-T1-01: should initialize ThemeContext with default light mode', () => {
    render(<ThemeTester />);
    expect(screen.getByTestId('current-theme')).toHaveTextContent('light');
    expect(screen.getByTestId('theme-container')).toHaveClass('light');
  });

  it('F2-T1-02: should toggle theme from light to dark using toggleTheme handler', async () => {
    const user = userEvent.setup();
    render(<ThemeTester />);

    const button = screen.getByRole('button', { name: /toggle theme/i });
    await user.click(button);

    expect(screen.getByTestId('current-theme')).toHaveTextContent('dark');
    expect(screen.getByTestId('theme-container')).toHaveClass('dark');
  });

  it('F2-T1-03: should apply dark class toggle on container element in dark mode', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
      },
    });

    const toggleButton = screen.getByRole('button', { name: /dark|light/i });
    expect(screen.getByTestId('main-app-container')).toHaveClass('light');

    await user.click(toggleButton);
    expect(screen.getByTestId('main-app-container')).toHaveClass('dark');
  });

  it('F2-T1-04: should support explicit initialTheme prop in ThemeProvider', () => {
    render(<ThemeTester />, {
      providerProps: {
        themeProps: { initialTheme: 'dark' },
      },
    });

    expect(screen.getByTestId('current-theme')).toHaveTextContent('dark');
    expect(screen.getByTestId('theme-container')).toHaveClass('dark');
  });

  it('F2-T1-05: should verify OKLCH color token variable structure and dot-grid paper substrate class', () => {
    render(<AppContent />);
    const rootContainer = screen.getByTestId('onboarding-wizard').parentElement;
    expect(rootContainer).toHaveClass('bg-dot-grid');
    expect(rootContainer).toHaveClass('bg-background');
    expect(rootContainer).toHaveClass('text-foreground');
  });
});
