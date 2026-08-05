import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { TrayPopup } from '@/components/tray/TrayPopup';
import { mockDrives } from '../mocks/vaultMocks';
import { mockTrayRecentActivity } from '../mocks/trayMocks';

describe('F8_TRAY: Refined 380px System Tray Popup', () => {
  it('F8-T1-01: should render compact 380px system tray popup window', () => {
    render(<TrayPopup />);
    const container = screen.getByTestId('tray-popup-container');
    expect(container).toBeInTheDocument();
    expect(container).toHaveClass('w-[380px]');
  });

  it('F8-T1-02: should render tray header bar with action icon buttons (↗ launch main window, ⚙ settings)', async () => {
    const user = userEvent.setup();
    render(<TrayPopup />);

    const launchBtn = screen.getByTestId('tray-launch-button');
    const settingsBtn = screen.getByTestId('tray-settings-button');

    expect(launchBtn).toBeInTheDocument();
    expect(settingsBtn).toBeInTheDocument();

    await user.click(launchBtn);
    await user.click(settingsBtn);
  });

  it('F8-T1-03: should render hero health status card with checkmark ✓ or progress ring', () => {
    render(<TrayPopup />, {
      providerProps: {
        telemetryProps: { initialIsSyncing: false },
      },
    });

    const heroHealth = screen.getByTestId('tray-hero-health');
    expect(heroHealth).toBeInTheDocument();
    expect(screen.getByText('✓')).toBeInTheDocument();
    expect(screen.getByText('Up to Date')).toBeInTheDocument();
  });

  it('F8-T1-04: should render drive stack cards with Infinite ∞ capacity', () => {
    render(<TrayPopup />, {
      providerProps: {
        vaultProps: { initialDrives: mockDrives },
      },
    });

    const driveStack = screen.getByTestId('tray-drive-stack');
    expect(driveStack).toBeInTheDocument();

    mockDrives.forEach((drive) => {
      const quotaElement = screen.getByTestId(`tray-quota-${drive.id}`);
      expect(quotaElement).toHaveTextContent('Infinite ∞');
    });
  });

  it('F8-T1-05: should render clean basename file activity stream with JetBrains Mono extension pills (RS, MD, DB)', () => {
    render(<TrayPopup />);

    const activityStream = screen.getByTestId('tray-activity-stream');
    expect(activityStream).toBeInTheDocument();

    mockTrayRecentActivity.forEach((activity) => {
      const item = screen.getByTestId(`tray-activity-item-${activity.id}`);
      expect(item).toBeInTheDocument();
      expect(item).toHaveTextContent(activity.name);
      expect(item).toHaveTextContent(activity.ext);
    });
  });
});
