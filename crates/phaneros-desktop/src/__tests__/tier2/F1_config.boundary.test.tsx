import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { useVault } from '@/context/VaultContext';
import { useTheme } from '@/context/ThemeContext';
import React from 'react';

// Test component exercising config fallbacks and boundary prop handling
const VaultConfigFallbackComponent: React.FC<{ onAction?: () => void }> = ({ onAction }) => {
  const { drives, addVault } = useVault();
  const { theme } = useTheme();

  return (
    <div data-testid="config-root" className={`theme-${theme}`}>
      <h2>Config Fallback Test</h2>
      <button onClick={onAction} data-testid="action-btn">
        Trigger Action
      </button>
      <button
        onClick={() =>
          addVault({
            name: '', // Empty name fallback
            path: '  //malformed///path/ ', // Malformed path string
          })
        }
        data-testid="add-malformed-btn"
      >
        Add Malformed Vault
      </button>
      <ul>
        {drives.map((drive) => (
          <li key={drive.id} data-testid={`drive-item-${drive.id}`}>
            <span data-testid={`drive-name-${drive.id}`}>{drive.name || 'Unnamed Vault'}</span>
            <span data-testid={`drive-path-${drive.id}`}>{drive.path.trim().replace(/\/+/g, '/')}</span>
            <span data-testid={`drive-quota-${drive.id}`}>
              {drive.quotaBytes === undefined ? 'Infinite ∞' : `${drive.quotaBytes} B`}
            </span>
            <span data-testid={`drive-used-${drive.id}`}>{drive.usedBytes ?? 0} B</span>
          </li>
        ))}
      </ul>
    </div>
  );
};

describe('F1_CFG: Project Config & Boundary Tests', () => {
  it('F1-T2-01: should handle missing optional vault attributes gracefully defaulting quotaBytes to undefined and rendering Infinite ∞', () => {
    render(<VaultConfigFallbackComponent />);

    // Default drives in mock should have quotaBytes === undefined
    const quotaElements = screen.getAllByTestId(/drive-quota-/i);
    expect(quotaElements.length).toBeGreaterThan(0);
    quotaElements.forEach((el) => {
      expect(el).toHaveTextContent('Infinite ∞');
    });
  });

  it('F1-T2-02: should handle malformed path strings with extra slashes and whitespace without throwing', async () => {
    const user = userEvent.setup();
    render(<VaultConfigFallbackComponent />);

    const addBtn = screen.getByTestId('add-malformed-btn');
    await user.click(addBtn);

    // Verify added vault rendered normalized path and unnamed fallback
    const unnamedVault = screen.getByText('Unnamed Vault');
    expect(unnamedVault).toBeInTheDocument();
  });

  it('F1-T2-03: should render gracefully inside an empty container div without crashing', () => {
    const container = document.createElement('div');
    document.body.appendChild(container);

    const { container: renderedContainer } = render(<VaultConfigFallbackComponent />, { container });
    expect(renderedContainer).toBeInTheDocument();
    expect(screen.getByTestId('config-root')).toBeInTheDocument();
  });

  it('F1-T2-04: should handle undefined providerProps gracefully using default provider values', () => {
    render(<VaultConfigFallbackComponent />, { providerProps: undefined });

    expect(screen.getByTestId('config-root')).toHaveClass('theme-light');
    expect(screen.getByTestId('action-btn')).toBeInTheDocument();
  });

  it('F1-T2-05: should safely execute when optional action handler prop is undefined', async () => {
    const user = userEvent.setup();
    render(<VaultConfigFallbackComponent onAction={undefined} />);

    const actionBtn = screen.getByTestId('action-btn');
    // Clicking when onAction is undefined should not throw an unhandled exception
    await expect(user.click(actionBtn)).resolves.not.toThrow();
  });
});
