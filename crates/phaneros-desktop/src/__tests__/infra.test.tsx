import { describe, it, expect } from 'vitest';
import { render, screen } from './helpers/render';
import { useTheme } from '@/context/ThemeContext';
import { useVault } from '@/context/VaultContext';
import { useTelemetry } from '@/context/TelemetryContext';
import { useOnboarding } from '@/context/OnboardingContext';
import { useView } from '@/context/ViewContext';
import { mockDrives } from './mocks/vaultMocks';
import { mockTelemetry } from './mocks/telemetryMocks';
import { mockFileTree } from './mocks/fileTreeMocks';
import { mockTextConflict, mockBinaryConflict } from './mocks/diffMocks';
import { mockTrayHealthSynced, mockTrayRecentActivity } from './mocks/trayMocks';
import { mockOnboardingStep1 } from './mocks/onboardingMocks';

const TestComponent = () => {
  const { theme, toggleTheme } = useTheme();
  const { activeDrive } = useVault();
  const { metrics } = useTelemetry();
  const { currentStep } = useOnboarding();
  const { activeTab } = useView();

  return (
    <div>
      <span data-testid="theme-value">{theme}</span>
      <button onClick={toggleTheme}>Toggle Theme</button>
      <span data-testid="active-drive">{activeDrive?.name}</span>
      <span data-testid="drive-quota">{activeDrive?.quotaBytes === undefined ? 'Infinite ∞' : activeDrive.quotaBytes}</span>
      <span data-testid="telemetry-speed">{metrics.transferSpeed}</span>
      <span data-testid="onboarding-step">{currentStep}</span>
      <span data-testid="active-tab">{activeTab}</span>
    </div>
  );
};

describe('Test Runner Infrastructure Verification', () => {
  it('should render components wrapped with all Context Providers via custom render helper', () => {
    render(<TestComponent />);

    expect(screen.getByTestId('theme-value')).toHaveTextContent('light');
    expect(screen.getByTestId('active-drive')).toHaveTextContent('default');
    expect(screen.getByTestId('drive-quota')).toHaveTextContent('Infinite ∞');
    expect(screen.getByTestId('telemetry-speed')).toHaveTextContent('4.2 MB/s');
    expect(screen.getByTestId('onboarding-step')).toHaveTextContent('1');
    expect(screen.getByTestId('active-tab')).toHaveTextContent('dashboard');
  });

  it('should enforce Infinite ∞ quota rule on mock vault fixtures', () => {
    mockDrives.forEach((drive) => {
      expect(drive.quotaBytes).toBeUndefined();
    });
  });

  it('should validate structure of telemetry, file tree, diff, tray, and onboarding mocks', () => {
    expect(mockTelemetry.deduplicationRatio).toBe('1.85×');
    expect(mockFileTree[0].children?.[0].badge).toBe('RS');
    expect(mockTextConflict.filename).toBe('README.md');
    expect(mockBinaryConflict.isBinary).toBe(true);
    expect(mockTrayHealthSynced.status).toBe('synced');
    expect(mockTrayRecentActivity[0].ext).toBe('RS');
    expect(mockOnboardingStep1.currentStep).toBe(1);
  });
});
