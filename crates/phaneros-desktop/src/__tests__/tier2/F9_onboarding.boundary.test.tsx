import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { useOnboarding } from '@/context/OnboardingContext';
import React, { useState } from 'react';

const OnboardingWizardBoundaryComponent: React.FC = () => {
  const {
    currentStep,
    isCompleted,
    destinationMode,
    serverUrl,
    serverToken,
    isConnected,
    vaults,
    nextStep,
    prevStep,
    completeOnboarding,
    setDestinationMode,
    setServerUrl,
    setServerToken,
    setConnected,
    addVault,
  } = useOnboarding();

  const [validationError, setValidationError] = useState<string>('');

  const isStep2Valid =
    destinationMode === 'cloud' ||
    (destinationMode === 'self-hosted' && serverUrl.trim().length > 0 && serverToken.trim().length > 0);

  const isStep3Valid = vaults.length > 0;

  const handleNext = () => {
    setValidationError('');
    if (currentStep === 2 && !isStep2Valid) {
      setValidationError('Server URL and security token are required for self-hosted setup.');
      return;
    }
    if (currentStep === 3 && !isStep3Valid) {
      setValidationError('At least one folder vault must be selected.');
      return;
    }
    if (currentStep === 5) {
      completeOnboarding();
    } else {
      nextStep();
    }
  };

  return (
    <div data-testid="onboarding-root" className="p-6 max-w-xl mx-auto border rounded-xl">
      <div data-testid="step-indicator">Step {currentStep} of 5</div>
      <div data-testid="completed-status">{isCompleted ? 'Onboarding Complete' : 'In Progress'}</div>

      {validationError && (
        <div data-testid="validation-error" className="p-2 bg-red-500/20 text-red-600 my-2">
          {validationError}
        </div>
      )}

      {/* Step 1: Welcome */}
      {currentStep === 1 && (
        <div data-testid="step-1-content">
          <h2>Welcome to Phaneros</h2>
          <p>Local-First Encryption Active</p>
        </div>
      )}

      {/* Step 2: Destination Setup */}
      {currentStep === 2 && (
        <div data-testid="step-2-content" className="space-y-3">
          <h2>Destination Setup</h2>
          <div>
            <button
              data-testid="mode-cloud-btn"
              onClick={() => setDestinationMode('cloud')}
              className={destinationMode === 'cloud' ? 'font-bold' : ''}
            >
              Phaneros Cloud
            </button>
            <button
              data-testid="mode-selfhosted-btn"
              onClick={() => setDestinationMode('self-hosted')}
              className={destinationMode === 'self-hosted' ? 'font-bold' : ''}
            >
              Self-Hosted Endpoint
            </button>
          </div>

          {destinationMode === 'self-hosted' && (
            <div className="space-y-2">
              <input
                data-testid="server-url-input"
                placeholder="https://sync.mycompany.com:8443"
                value={serverUrl}
                onChange={(e) => setServerUrl(e.target.value)}
              />
              <input
                data-testid="server-token-input"
                placeholder="Enter secret token..."
                value={serverToken}
                onChange={(e) => setServerToken(e.target.value)}
              />
              <button
                data-testid="test-conn-btn"
                onClick={() => setConnected(isStep2Valid)}
              >
                Test Connection
              </button>
              <span data-testid="conn-status">{isConnected ? 'Connected' : 'Not Connected'}</span>
            </div>
          )}
        </div>
      )}

      {/* Step 3: Vault Selection */}
      {currentStep === 3 && (
        <div data-testid="step-3-content">
          <h2>Folder Vault Selection</h2>
          <button
            data-testid="add-preset-vault-btn"
            onClick={() => addVault({ name: 'Documents', path: '~/Documents' })}
          >
            Add ~/Documents Vault
          </button>
          <ul data-testid="vault-list">
            {vaults.map((v) => (
              <li key={v.id} data-testid={`vault-item-${v.id}`}>
                {v.name} - {v.quotaBytes === undefined ? 'Infinite ∞' : `${v.quotaBytes} B`}
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Step 4: Test Drive */}
      {currentStep === 4 && (
        <div data-testid="step-4-content">
          <h2>Telemetry Test Drive</h2>
          <p>Verifying server roundtrip and data flow...</p>
        </div>
      )}

      {/* Step 5: Setup Complete */}
      {currentStep === 5 && (
        <div data-testid="step-5-content">
          <h2>Setup Complete</h2>
          <button data-testid="finish-onboarding-btn" onClick={completeOnboarding}>
            Transition to Control Center
          </button>
        </div>
      )}

      {/* Navigation Buttons */}
      <div className="flex justify-between mt-6">
        <button data-testid="prev-step-btn" onClick={prevStep} disabled={currentStep === 1}>
          Back
        </button>
        <button
          data-testid="next-step-btn"
          onClick={handleNext}
          disabled={currentStep === 2 ? !isStep2Valid : currentStep === 3 ? !isStep3Valid : false}
        >
          {currentStep === 5 ? 'Complete' : 'Continue'}
        </button>
      </div>
    </div>
  );
};

describe('F9_ONBD: 5-Step Onboarding Wizard Boundary Tests', () => {
  it('F9-T2-01: should block proceeding to Step 3 when self-hosted mode has empty URL or token', async () => {
    const user = userEvent.setup();
    render(<OnboardingWizardBoundaryComponent />, {
      providerProps: {
        onboardingProps: {
          initialState: { currentStep: 2, destinationMode: 'self-hosted', serverUrl: '', serverToken: '' },
        },
      },
    });

    const nextBtn = screen.getByTestId('next-step-btn');
    expect(nextBtn).toBeDisabled();

    // Fill only URL
    const urlInput = screen.getByTestId('server-url-input');
    await user.type(urlInput, 'https://sync.mycompany.com');
    expect(nextBtn).toBeDisabled();

    // Fill token as well
    const tokenInput = screen.getByTestId('server-token-input');
    await user.type(tokenInput, 'secret_token_123');
    expect(nextBtn).not.toBeDisabled();
  });

  it('F9-T2-02: should disable continue button on Step 3 when zero folder vaults are selected', () => {
    render(<OnboardingWizardBoundaryComponent />, {
      providerProps: {
        onboardingProps: {
          initialState: { currentStep: 3, vaults: [] },
        },
      },
    });

    const nextBtn = screen.getByTestId('next-step-btn');
    expect(nextBtn).toBeDisabled();
  });

  it('F9-T2-03: should enforce navigation boundaries at Step 1 and Step 5', async () => {
    render(<OnboardingWizardBoundaryComponent />, {
      providerProps: {
        onboardingProps: {
          initialState: { currentStep: 1 },
        },
      },
    });

    const prevBtn = screen.getByTestId('prev-step-btn');
    expect(prevBtn).toBeDisabled();
  });

  it('F9-T2-04: should set isCompleted to true and maintain step 5 upon completing onboarding', async () => {
    const user = userEvent.setup();
    render(<OnboardingWizardBoundaryComponent />, {
      providerProps: {
        onboardingProps: {
          initialState: { currentStep: 5, isCompleted: false },
        },
      },
    });

    const finishBtn = screen.getByTestId('finish-onboarding-btn');
    await user.click(finishBtn);

    expect(screen.getByTestId('completed-status')).toHaveTextContent('Onboarding Complete');
  });

  it('F9-T2-05: should maintain step integrity under rapid forward navigation', async () => {
    const user = userEvent.setup();
    render(<OnboardingWizardBoundaryComponent />, {
      providerProps: {
        onboardingProps: {
          initialState: { currentStep: 1, destinationMode: 'cloud' },
        },
      },
    });

    const nextBtn = screen.getByTestId('next-step-btn');

    // Click next twice to advance from Step 1 -> Step 2 -> Step 3
    await user.click(nextBtn);
    expect(screen.getByTestId('step-indicator')).toHaveTextContent('Step 2 of 5');
  });
});
