import React from 'react';
import { useOnboarding } from '@/context/OnboardingContext';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { mockPresetVaults } from '@/__tests__/mocks/vaultMocks';

export const OnboardingWizard: React.FC = () => {
  const {
    currentStep,
    setStep,
    nextStep,
    prevStep,
    completeOnboarding,
    destinationMode,
    setDestinationMode,
    serverUrl,
    setServerUrl,
    serverToken,
    setServerToken,
    isConnected,
    setConnected,
    vaults,
    addVault,
    removeVault,
  } = useOnboarding();

  const handleTestConnection = () => {
    if (destinationMode === 'self-hosted' && (!serverUrl || !serverToken)) {
      setConnected(false);
      return;
    }
    setConnected(true);
  };

  return (
    <div className="flex flex-col items-center justify-center min-h-[600px] max-w-2xl mx-auto p-6 font-sans" data-testid="onboarding-wizard">
      {/* Stepper Header */}
      <div className="w-full flex items-center justify-between mb-8 pb-4 border-b border-border" data-testid="onboarding-stepper">
        {[1, 2, 3, 4, 5].map((stepNum) => {
          const isActive = currentStep === stepNum;
          const isDone = currentStep > stepNum;
          return (
            <div
              key={stepNum}
              onClick={() => setStep(stepNum)}
              className={`flex items-center gap-2 cursor-pointer ${
                isActive ? 'text-primary font-bold' : isDone ? 'text-emerald-600 font-semibold' : 'text-muted-foreground'
              }`}
              data-testid={`stepper-step-${stepNum}`}
            >
              <div
                className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-mono border ${
                  isActive
                    ? 'border-primary bg-primary text-primary-foreground'
                    : isDone
                    ? 'border-emerald-600 bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300'
                    : 'border-border bg-muted'
                }`}
              >
                {isDone ? '✓' : stepNum}
              </div>
              <span className="text-xs hidden sm:inline">Step {stepNum}</span>
            </div>
          );
        })}
      </div>

      {/* Step Content */}
      <Card elevation="medium" className="w-full p-6 flex flex-col gap-6 bg-card" data-testid={`onboarding-step-content-${currentStep}`}>
        {/* Step 1: Welcome & Local-First */}
        {currentStep === 1 && (
          <div className="flex flex-col gap-4 text-center" data-testid="step-1-welcome">
            <h2 className="text-2xl font-serif font-bold text-foreground">Welcome to Phaneros</h2>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Your local-first, zero-trust file synchronization control center.
              All data is locally encrypted before leaving your machine.
            </p>
            <div className="p-4 bg-muted/40 rounded-xl border border-border flex items-center justify-center gap-3">
              <span className="text-2xl">🔒</span>
              <span className="text-xs font-semibold text-foreground">End-to-End Local Encryption Enabled</span>
            </div>
          </div>
        )}

        {/* Step 2: Destination Setup */}
        {currentStep === 2 && (
          <div className="flex flex-col gap-4" data-testid="step-2-destination">
            <h2 className="text-xl font-serif font-bold text-foreground">Choose Synchronization Destination</h2>
            <div className="grid grid-cols-2 gap-4">
              <button
                type="button"
                onClick={() => setDestinationMode('cloud')}
                className={`p-4 rounded-xl border text-left transition-all ${
                  destinationMode === 'cloud' ? 'border-primary bg-accent/40 font-semibold' : 'border-border bg-card'
                }`}
                data-testid="mode-cloud-button"
              >
                <div className="font-bold text-sm">Phaneros Managed Cloud</div>
                <div className="text-xs text-muted-foreground mt-1">Zero-configuration cloud endpoint.</div>
              </button>

              <button
                type="button"
                onClick={() => setDestinationMode('self-hosted')}
                className={`p-4 rounded-xl border text-left transition-all ${
                  destinationMode === 'self-hosted' ? 'border-primary bg-accent/40 font-semibold' : 'border-border bg-card'
                }`}
                data-testid="mode-self-hosted-button"
              >
                <div className="font-bold text-sm">Self-Hosted Server</div>
                <div className="text-xs text-muted-foreground mt-1">Connect your private server URL & token.</div>
              </button>
            </div>

            {destinationMode === 'self-hosted' && (
              <div className="flex flex-col gap-3 mt-2 p-4 bg-muted/40 rounded-xl border border-border">
                <div className="flex flex-col gap-1">
                  <label className="text-xs font-semibold">Server URL</label>
                  <input
                    type="text"
                    value={serverUrl}
                    onChange={(e) => setServerUrl(e.target.value)}
                    placeholder="https://sync.company.com:8443"
                    className="p-2 text-xs font-mono rounded border border-border bg-background"
                    data-testid="input-server-url"
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-xs font-semibold">Security Token</label>
                  <input
                    type="password"
                    value={serverToken}
                    onChange={(e) => setServerToken(e.target.value)}
                    placeholder="sec_token_..."
                    className="p-2 text-xs font-mono rounded border border-border bg-background"
                    data-testid="input-server-token"
                  />
                </div>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={handleTestConnection}
                  data-testid="test-connection-button"
                >
                  Test Endpoint Connection
                </Button>
                {isConnected && (
                  <span className="text-xs font-semibold text-emerald-600 dark:text-emerald-400" data-testid="connection-status-success">
                    ✓ Connection Successful
                  </span>
                )}
              </div>
            )}
          </div>
        )}

        {/* Step 3: Folder Vault Selection */}
        {currentStep === 3 && (
          <div className="flex flex-col gap-4" data-testid="step-3-vaults">
            <h2 className="text-xl font-serif font-bold text-foreground">Select Local Folder Vaults</h2>
            <p className="text-sm text-muted-foreground">Pick preset folders or custom directories to sync.</p>

            <div className="grid grid-cols-2 gap-2">
              {mockPresetVaults.map((preset) => {
                const isAdded = vaults.some((v) => v.name === preset.name);
                return (
                  <Button
                    key={preset.name}
                    variant={isAdded ? 'primary' : 'outline'}
                    size="sm"
                    onClick={() => {
                      if (isAdded) {
                        const target = vaults.find((v) => v.name === preset.name);
                        if (target) removeVault(target.id);
                      } else {
                        addVault({ name: preset.name, path: preset.path });
                      }
                    }}
                    data-testid={`preset-vault-${preset.name}`}
                  >
                    {isAdded ? `✓ ${preset.name}` : `+ ${preset.name}`}
                  </Button>
                );
              })}
            </div>

            <div className="flex flex-col gap-2 mt-2">
              <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Selected Vaults</span>
              {vaults.map((vault) => (
                <div key={vault.id} className="p-3 bg-muted/40 rounded-lg border border-border flex items-center justify-between text-xs">
                  <div>
                    <span className="font-semibold">{vault.name}</span>
                    <span className="text-muted-foreground ml-2 font-mono">{vault.path}</span>
                  </div>
                  <span className="font-mono font-bold text-foreground" data-testid={`onboarding-vault-quota-${vault.id}`}>
                    {vault.quotaBytes === undefined ? 'Infinite ∞' : `${vault.quotaBytes} B`}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Step 4: Connection & Telemetry Test Drive */}
        {currentStep === 4 && (
          <div className="flex flex-col gap-4 text-center" data-testid="step-4-test-drive">
            <h2 className="text-xl font-serif font-bold text-foreground">Telemetry & Connection Test Drive</h2>
            <p className="text-sm text-muted-foreground">Simulating network roundtrip and local encryption test drive.</p>
            <div className="p-6 bg-muted/40 rounded-xl border border-border flex flex-col items-center gap-3">
              <div className="w-full bg-border h-2 rounded-full overflow-hidden">
                <div className="bg-primary h-full w-3/4 animate-pulse" data-testid="test-drive-progress-bar" />
              </div>
              <span className="text-xs font-mono font-semibold text-foreground">Verifying server handshake: 12.4 MB/s</span>
            </div>
          </div>
        )}

        {/* Step 5: Finish & Launch */}
        {currentStep === 5 && (
          <div className="flex flex-col gap-4 text-center" data-testid="step-5-finish">
            <h2 className="text-2xl font-serif font-bold text-foreground">Setup Complete!</h2>
            <p className="text-sm text-muted-foreground">
              Phaneros Shield Core is active. Your vaults are monitored and protected.
            </p>
            <Button
              variant="primary"
              size="lg"
              onClick={completeOnboarding}
              data-testid="finish-and-launch-button"
            >
              Launch Control Center
            </Button>
          </div>
        )}

        {/* Wizard Footer Navigation Controls */}
        <div className="flex justify-between items-center pt-4 border-t border-border mt-2">
          {currentStep > 1 ? (
            <Button variant="outline" size="sm" onClick={prevStep} data-testid="wizard-prev-button">
              Previous
            </Button>
          ) : (
            <div />
          )}

          {currentStep < 5 && (
            <Button variant="primary" size="sm" onClick={nextStep} data-testid="wizard-next-button">
              Next Step
            </Button>
          )}
        </div>
      </Card>
    </div>
  );
};
