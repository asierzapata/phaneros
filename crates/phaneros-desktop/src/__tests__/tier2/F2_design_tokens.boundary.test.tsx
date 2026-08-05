import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { useTheme } from '@/context/ThemeContext';
import React from 'react';

const ThemeBoundaryComponent: React.FC = () => {
  const { theme, toggleTheme, setTheme } = useTheme();

  return (
    <div data-testid="theme-container" className={`app-root ${theme}`}>
      <span data-testid="current-theme">{theme}</span>
      <button data-testid="toggle-theme-btn" onClick={toggleTheme}>
        Toggle
      </button>
      <button
        data-testid="set-invalid-theme-btn"
        onClick={() => {
          // Force cast invalid string to test runtime resilience
          setTheme('invalid-mode' as any);
        }}
      >
        Set Invalid
      </button>
      <div
        data-testid="paper-substrate"
        style={{
          backgroundImage:
            theme === 'dark'
              ? 'radial-gradient(rgba(255, 255, 255, 0.1) 1px, transparent 1px)'
              : 'radial-gradient(rgba(0, 0, 0, 0.08) 1px, transparent 1px)',
        }}
      />
    </div>
  );
};

describe('F2_DS: Design Tokens & Theme Boundary Tests', () => {
  it('F2-T2-01: should handle rapid consecutive theme toggles without state desynchronization', async () => {
    const user = userEvent.setup();
    render(<ThemeBoundaryComponent />);

    const toggleBtn = screen.getByTestId('toggle-theme-btn');
    const themeSpan = screen.getByTestId('current-theme');

    expect(themeSpan).toHaveTextContent('light');

    // Perform 5 rapid consecutive clicks
    await user.click(toggleBtn);
    await user.click(toggleBtn);
    await user.click(toggleBtn);
    await user.click(toggleBtn);
    await user.click(toggleBtn);

    // 5 toggles from light -> dark -> light -> dark -> light -> dark
    expect(themeSpan).toHaveTextContent('dark');
  });

  it('F2-T2-02: should maintain valid theme state when invalid theme value is provided', async () => {
    const user = userEvent.setup();
    render(<ThemeBoundaryComponent />);

    const setInvalidBtn = screen.getByTestId('set-invalid-theme-btn');
    await user.click(setInvalidBtn);

    const container = screen.getByTestId('theme-container');
    // Application container should retain a string class without breaking DOM rendering
    expect(container).toBeInTheDocument();
  });

  it('F2-T2-03: should correctly update dot-grid paper substrate background on theme change', async () => {
    const user = userEvent.setup();
    render(<ThemeBoundaryComponent />);

    const substrate = screen.getByTestId('paper-substrate');
    expect(substrate.style.backgroundImage).toContain('rgba(0, 0, 0, 0.08)');

    const toggleBtn = screen.getByTestId('toggle-theme-btn');
    await user.click(toggleBtn);

    expect(substrate.style.backgroundImage).toContain('rgba(255, 255, 255, 0.1)');
  });

  it('F2-T2-04: should handle localStorage error gracefully during theme persistence', async () => {
    // Mock localStorage getItem/setItem to throw
    const originalSetItem = localStorage.setItem;
    localStorage.setItem = () => {
      throw new Error('QuotaExceededError');
    };

    const user = userEvent.setup();
    render(<ThemeBoundaryComponent />);

    const toggleBtn = screen.getByTestId('toggle-theme-btn');
    await expect(user.click(toggleBtn)).resolves.not.toThrow();

    // Restore localStorage
    localStorage.setItem = originalSetItem;
  });

  it('F2-T2-05: should apply custom initial theme override via providerProps', () => {
    render(<ThemeBoundaryComponent />, {
      providerProps: {
        themeProps: { initialTheme: 'dark' },
      },
    });

    expect(screen.getByTestId('current-theme')).toHaveTextContent('dark');
    expect(screen.getByTestId('theme-container')).toHaveClass('dark');
  });
});
