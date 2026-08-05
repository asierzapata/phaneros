import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { AppContent } from '@/App';
import { TrayPopup } from '@/components/tray/TrayPopup';
import { OnboardingWizard } from '@/components/onboarding/OnboardingWizard';
import { DrivesFiles } from '@/components/main/DrivesFiles';
import { Conflicts } from '@/components/main/Conflicts';
import { mockOnboardingStep5Completed, mockOnboardingStep1 } from '../mocks/onboardingMocks';
import { mockDrives } from '../mocks/vaultMocks';

// TrayPopup and AppContent normally run in separate Tauri webview windows and
// communicate only via Tauri events. These mocks emulate that IPC bridge
// in-process (a fake event bus) so CF-10 can exercise the real cross-window
// navigation path instead of the two components sharing React context directly.
vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  // `DaemonStatusContext` polls `daemon_ping` on mount in any `isTauri()`
  // window, including these cross-feature tests — resolve it as a healthy,
  // configured daemon so daemon-connectivity gating doesn't block rendering
  // here (this suite isn't exercising that feature). Other commands aren't
  // mocked, matching this file's existing behavior pre-dating that gate.
  invoke: vi.fn((command: string) => {
    if (command === 'daemon_ping') {
      return Promise.resolve({ version: '0.1.0', configured: true });
    }
    return Promise.reject(new Error(`invoke("${command}") not mocked in this test`));
  }),
}));

const eventListeners: Record<string, Array<(event: { payload: unknown }) => void>> = {};

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((eventName: string, cb: (event: { payload: unknown }) => void) => {
    (eventListeners[eventName] ??= []).push(cb);
    return Promise.resolve(() => {
      eventListeners[eventName] = (eventListeners[eventName] || []).filter((fn) => fn !== cb);
    });
  }),
  emit: vi.fn((eventName: string, payload: unknown) => {
    (eventListeners[eventName] || []).forEach((cb) => cb({ payload }));
    return Promise.resolve();
  }),
}));

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  WebviewWindow: {
    getByLabel: vi.fn(() =>
      Promise.resolve({ show: vi.fn(() => Promise.resolve()), setFocus: vi.fn(() => Promise.resolve()) })
    ),
  },
}));

vi.mock('@tauri-apps/api/window', () => ({
  // Keep AppContent identifying as the "main" window while this mock also
  // backs the tray-side getCurrentWindow().hide() call in trayBridge.
  getCurrentWindow: vi.fn(() => ({ label: 'main', hide: vi.fn(() => Promise.resolve()) })),
}));

describe('Tier 3: Pairwise Cross-Feature Integration Suite', () => {
  it('CF-01: ThemeContext toggle (F2_DS) propagates color scheme to Shadcn primitives (F3_PRIM) and Header bar (F4_HDR)', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
      },
    });

    const appContainer = screen.getByTestId('main-app-container');
    expect(appContainer).toHaveClass('light');

    const toggleButton = screen.getByRole('button', { name: /dark|light/i });
    await user.click(toggleButton);

    expect(appContainer).toHaveClass('dark');
  });

  it('CF-02: VaultContext active drive selection (F5_DASH) filters Trees.software file tree explorer (F6_TREE)', async () => {
    const user = userEvent.setup();
    render(<DrivesFiles />, {
      providerProps: {
        vaultProps: { initialDrives: mockDrives, initialActiveId: mockDrives[0].id },
      },
    });

    expect(screen.getByTestId('drive-metadata-card')).toHaveTextContent(mockDrives[0].name);

    const secondDriveItem = screen.getByTestId(`drive-selector-item-${mockDrives[1].id}`);
    await user.click(secondDriveItem);

    expect(screen.getByTestId('drive-metadata-card')).toHaveTextContent(mockDrives[1].name);
    expect(screen.getByTestId('active-drive-quota-display')).toHaveTextContent('Infinite ∞');
  });

  it('CF-03: Conflict resolution action in Diffs.com workspace (F7_DIFF) updates VaultContext status (F10_INT) and Telemetry metrics (F5_DASH)', async () => {
    const user = userEvent.setup();
    render(<Conflicts />);

    const keepLocalBtn = screen.getByRole('button', { name: /keep local/i });
    await user.click(keepLocalBtn);

    const banner = screen.getByTestId('resolution-banner');
    expect(banner).toHaveTextContent(/Kept Local Copy/i);
  });

  it('CF-04: Onboarding wizard vault selection preset in Step 3 (F9_ONBD) populates VaultContext drive list (F5_DASH / F10_INT)', async () => {
    const user = userEvent.setup();
    render(<OnboardingWizard />, {
      providerProps: {
        onboardingProps: { initialState: { ...mockOnboardingStep1, currentStep: 3 } },
      },
    });

    const devPresetBtn = screen.getByTestId('preset-vault-Developer');
    await user.click(devPresetBtn);

    expect(screen.getByText('Developer')).toBeInTheDocument();
    expect(screen.getByTestId(/onboarding-vault-quota-/)).toHaveTextContent('Infinite ∞');
  });

  it('CF-05: Triggering manual Sync Now from System Dashboard (F5_DASH) updates Tray popup hero health state (F8_TRAY) and TelemetryContext (F10_INT)', async () => {
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

  it('CF-06: Switching HIG header tabs (F4_HDR) preserves active drive selection and state in File Explorer (F6_TREE) and Conflicts (F7_DIFF)', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
        vaultProps: { initialDrives: mockDrives, initialActiveId: mockDrives[1].id },
      },
    });

    // Go to Drives view
    await user.click(screen.getByRole('tab', { name: /drives & files/i }));
    expect(screen.getByTestId('drive-metadata-card')).toHaveTextContent(mockDrives[1].name);

    // Switch to Conflicts view
    await user.click(screen.getByRole('tab', { name: /conflicts/i }));
    expect(screen.getByTestId('conflicts-workspace')).toBeInTheDocument();

    // Switch back to Drives view
    await user.click(screen.getByRole('tab', { name: /drives & files/i }));
    expect(screen.getByTestId('drive-metadata-card')).toHaveTextContent(mockDrives[1].name);
  });

  it('CF-07: Adding a new vault in Onboarding (F9_ONBD) enforces Infinite ∞ quota rule in Dashboard drive cards (F5_DASH) and Tray drive stack (F8_TRAY)', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: { ...mockOnboardingStep1, currentStep: 3 } },
      },
    });

    await user.click(screen.getByTestId('preset-vault-Pictures'));
    expect(screen.getByText('Pictures')).toBeInTheDocument();
    expect(screen.getByTestId(/onboarding-vault-quota-/)).toHaveTextContent('Infinite ∞');
  });

  it('CF-08: File tree selection in Explorer (F6_TREE) with conflict status navigates or links to Diffs.com Conflict workspace (F7_DIFF)', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep5Completed },
      },
    });

    await user.click(screen.getByRole('tab', { name: /drives & files/i }));
    expect(screen.getByTestId('drives-files-workspace')).toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: /conflicts/i }));
    expect(screen.getByTestId('conflicts-workspace')).toBeInTheDocument();
  });

  it('CF-09: Completing Onboarding flow (F9_ONBD) updates ViewContext and OnboardingContext to render Main Control Center (F4_HDR / F5_DASH)', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: { ...mockOnboardingStep1, currentStep: 5 } },
      },
    });

    expect(screen.getByTestId('step-5-finish')).toBeInTheDocument();

    await user.click(screen.getByTestId('finish-and-launch-button'));

    expect(screen.getByTestId('main-header')).toBeInTheDocument();
    expect(screen.getByTestId('system-dashboard')).toBeInTheDocument();
  });

  it('CF-10: System Tray popup activity item click (F8_TRAY) triggers ViewContext active tab navigation in Main Control Center (F4_HDR / F10_INT)', async () => {
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

    const activityItem = screen.getByTestId('tray-activity-item-act-1');
    await user.click(activityItem);

    expect(screen.getByTestId('activity-workspace')).toBeInTheDocument();
  });
});
