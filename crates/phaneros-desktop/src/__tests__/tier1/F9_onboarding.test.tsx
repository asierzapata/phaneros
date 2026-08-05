import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { OnboardingWizard } from '@/components/onboarding/OnboardingWizard';
import { AppContent } from '@/App';
import { mockOnboardingStep1 } from '../mocks/onboardingMocks';

describe('F9_ONBD: 5-Step Onboarding Wizard', () => {
  it('F9-T1-01: should render Step 1 Welcome screen with local-first protection copy', () => {
    render(<OnboardingWizard />, {
      providerProps: {
        onboardingProps: { initialState: mockOnboardingStep1 },
      },
    });

    expect(screen.getByTestId('step-1-welcome')).toBeInTheDocument();
    expect(screen.getByText(/Welcome to Phaneros/i)).toBeInTheDocument();
    expect(screen.getByText(/End-to-End Local Encryption Enabled/i)).toBeInTheDocument();
  });

  it('F9-T1-02: should render Step 2 Destination Setup supporting Cloud vs Self-Hosted URL+Token testing', async () => {
    const user = userEvent.setup();
    render(<OnboardingWizard />, {
      providerProps: {
        onboardingProps: { initialState: { ...mockOnboardingStep1, currentStep: 2 } },
      },
    });

    expect(screen.getByTestId('step-2-destination')).toBeInTheDocument();

    const selfHostedBtn = screen.getByTestId('mode-self-hosted-button');
    await user.click(selfHostedBtn);

    const urlInput = screen.getByTestId('input-server-url');
    const tokenInput = screen.getByTestId('input-server-token');
    const testBtn = screen.getByTestId('test-connection-button');

    await user.type(urlInput, 'https://sync.mycorp.com:8443');
    await user.type(tokenInput, 'secret_token_123');
    await user.click(testBtn);

    expect(screen.getByTestId('connection-status-success')).toBeInTheDocument();
  });

  it('F9-T1-03: should render Step 3 Folder Vault Selection with quick presets and Infinite ∞ capacity', async () => {
    const user = userEvent.setup();
    render(<OnboardingWizard />, {
      providerProps: {
        onboardingProps: { initialState: { ...mockOnboardingStep1, currentStep: 3 } },
      },
    });

    expect(screen.getByTestId('step-3-vaults')).toBeInTheDocument();

    const presetDocsBtn = screen.getByTestId('preset-vault-Documents');
    await user.click(presetDocsBtn);

    expect(screen.getByText('Documents')).toBeInTheDocument();
    expect(screen.getByText('Infinite ∞')).toBeInTheDocument();
  });

  it('F9-T1-04: should render Step 4 Connection & Telemetry Test Drive with surge animation', () => {
    render(<OnboardingWizard />, {
      providerProps: {
        onboardingProps: { initialState: { ...mockOnboardingStep1, currentStep: 4 } },
      },
    });

    expect(screen.getByTestId('step-4-test-drive')).toBeInTheDocument();
    expect(screen.getByTestId('test-drive-progress-bar')).toBeInTheDocument();
  });

  it('F9-T1-05: should render Step 5 Finish & Launch screen and trigger transition to Main Control Center', async () => {
    const user = userEvent.setup();
    render(<AppContent />, {
      providerProps: {
        onboardingProps: { initialState: { ...mockOnboardingStep1, currentStep: 5 } },
      },
    });

    expect(screen.getByTestId('step-5-finish')).toBeInTheDocument();

    const launchBtn = screen.getByTestId('finish-and-launch-button');
    await user.click(launchBtn);

    expect(screen.getByTestId('main-header')).toBeInTheDocument();
    expect(screen.getByTestId('system-dashboard')).toBeInTheDocument();
  });
});
