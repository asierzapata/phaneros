import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { AppContent } from '@/App';
import { TrayPopup } from '@/components/tray/TrayPopup';
import { mockOnboardingStep5Completed } from '../mocks/onboardingMocks';
import { mockDrives } from '../mocks/vaultMocks';

describe('F10_INT: State Integration & React Context', () => {
  it('F10-T1-01: should integrate ThemeContext with all components across application', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
      },
    });

    const container = screen.getByTestId('main-app-container');
    expect(container).toHaveClass('light');

    const toggleBtn = screen.getByRole('button', { name: /dark|light/i });
    await user.click(toggleBtn);

    expect(container).toHaveClass('dark');
  });

  it('F10-T1-02: should integrate VaultContext for drive selection and infinite quota calculation across views', () => {
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
        vaultProps: { initialDrives: mockDrives },
      },
    });

    mockDrives.forEach((drive) => {
      expect(screen.getByTestId(`quota-display-${drive.id}`)).toHaveTextContent('Infinite ∞');
    });
  });

  it('F10-T1-03: should integrate TelemetryContext sync state updates across Dashboard and Tray', async () => {
    const user = userEvent.setup();
    render(
      <div>
        <AppContent />
        <TrayPopup />
      </div>,
      {
        providerProps: {
          onboardingProps: { initialState: mockOnboardingStep5Completed },
        },
      }
    );

    const syncButton = screen.getByTestId('sync-now-button');
    await user.click(syncButton);

    expect(screen.getByTestId('tray-hero-health')).toHaveTextContent('Syncing Vaults...');
  });

  it('F10-T1-04: should integrate OnboardingContext step navigation and completion state', async () => {
    const user = userEvent.setup();
    render(<AppContent />);

    expect(screen.getByTestId('onboarding-wizard')).toBeInTheDocument();

    const nextBtn = screen.getByTestId('wizard-next-button');
    await user.click(nextBtn);

    expect(screen.getByTestId('step-2-destination')).toBeInTheDocument();
  });

  it('F10-T1-05: should integrate ViewContext root routing between onboarding, main control center, and tray views', async () => {
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
  });
});
