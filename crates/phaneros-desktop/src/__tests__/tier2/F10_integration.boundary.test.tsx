import { describe, it, expect } from 'vitest';
import { render, screen, renderHook } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { useVault } from '@/context/VaultContext';
import { useOnboarding } from '@/context/OnboardingContext';
import { useTheme } from '@/context/ThemeContext';
import { useTelemetry } from '@/context/TelemetryContext';
import { useView } from '@/context/ViewContext';
import { useActivity } from '@/context/ActivityContext';
import React from 'react';

const GlobalStateIntegrationComponent: React.FC = () => {
  const { drives, addVault: addVaultGlobal } = useVault();
  const { vaults: onboardingVaults, addVault: addVaultOnboarding } = useOnboarding();
  const { theme, toggleTheme } = useTheme();
  const { isSyncing, triggerSync } = useTelemetry();
  const { activeTab, setActiveTab } = useView();
  const { activity } = useActivity();

  const handleCrossAdd = () => {
    const newVault = { name: 'CrossVault', path: '~/Documents/CrossVault' };
    addVaultGlobal(newVault);
    addVaultOnboarding(newVault);
  };

  const handleConcurrentUpdate = () => {
    addVaultGlobal({ name: 'ConcurrentVault', path: '~/Documents/Concurrent' });
    triggerSync();
    setActiveTab('conflicts');
  };

  return (
    <div data-testid="integration-root" className={`theme-${theme}`}>
      <span data-testid="global-theme">{theme}</span>
      <span data-testid="active-tab">{activeTab}</span>
      <span data-testid="sync-status">{isSyncing ? 'Syncing' : 'Idle'}</span>
      <span data-testid="activity-count">{activity.length}</span>

      <button data-testid="toggle-theme-btn" onClick={toggleTheme}>
        Toggle Theme
      </button>

      <button data-testid="cross-add-btn" onClick={handleCrossAdd}>
        Add Vault Everywhere
      </button>

      <button data-testid="concurrent-update-btn" onClick={handleConcurrentUpdate}>
        Trigger Concurrent Update
      </button>

      <div data-testid="global-drives-list">
        {drives.map((d) => (
          <div key={d.id} data-testid={`global-drive-${d.name}`}>
            {d.name} - {d.quotaBytes === undefined ? 'Infinite ∞' : `${d.quotaBytes} B`}
          </div>
        ))}
      </div>

      <div data-testid="onboarding-drives-list">
        {onboardingVaults.map((v) => (
          <div key={v.id} data-testid={`onboarding-drive-${v.name}`}>
            {v.name} - {v.quotaBytes === undefined ? 'Infinite ∞' : `${v.quotaBytes} B`}
          </div>
        ))}
      </div>
    </div>
  );
};

describe('F10_INT: State Integration & React Context Boundary Tests', () => {
  it('F10-T2-01: should synchronize vault additions between OnboardingContext and VaultContext', async () => {
    const user = userEvent.setup();
    render(<GlobalStateIntegrationComponent />);

    const crossAddBtn = screen.getByTestId('cross-add-btn');
    await user.click(crossAddBtn);

    expect(screen.getByTestId('global-drive-CrossVault')).toHaveTextContent('CrossVault - Infinite ∞');
    expect(screen.getByTestId('onboarding-drive-CrossVault')).toHaveTextContent('CrossVault - Infinite ∞');
  });

  it('F10-T2-02: should propagate theme toggle across all integrated sub-components', async () => {
    const user = userEvent.setup();
    render(<GlobalStateIntegrationComponent />);

    expect(screen.getByTestId('global-theme')).toHaveTextContent('light');

    const toggleBtn = screen.getByTestId('toggle-theme-btn');
    await user.click(toggleBtn);

    expect(screen.getByTestId('global-theme')).toHaveTextContent('dark');
    expect(screen.getByTestId('integration-root')).toHaveClass('theme-dark');
  });

  it('F10-T2-03: should throw descriptive error when hook is rendered outside provider', () => {
    // Attempting to render hook without provider wrapper should throw
    expect(() => {
      renderHook(() => useTheme());
    }).toThrow('useTheme must be used within a ThemeProvider');
  });

  it('F10-T2-04: should maintain default initial tab and theme states when resetting context', () => {
    render(<GlobalStateIntegrationComponent />);

    expect(screen.getByTestId('active-tab')).toHaveTextContent('dashboard');
    expect(screen.getByTestId('global-theme')).toHaveTextContent('light');
  });

  it('F10-T2-06: should default ActivityContext to mock recent activity outside Tauri', () => {
    render(<GlobalStateIntegrationComponent />);

    expect(screen.getByTestId('activity-count')).not.toHaveTextContent('0');
  });

  it('F10-T2-07: should throw descriptive error when useActivity is rendered outside provider', () => {
    expect(() => {
      renderHook(() => useActivity());
    }).toThrow('useActivity must be used within an ActivityProvider');
  });

  it('F10-T2-05: should handle concurrent multi-context updates without state corruption', async () => {
    const user = userEvent.setup();
    render(<GlobalStateIntegrationComponent />);

    const concurrentBtn = screen.getByTestId('concurrent-update-btn');
    await user.click(concurrentBtn);

    expect(screen.getByTestId('global-drive-ConcurrentVault')).toBeInTheDocument();
    expect(screen.getByTestId('sync-status')).toHaveTextContent('Syncing');
    expect(screen.getByTestId('active-tab')).toHaveTextContent('conflicts');
  });
});
