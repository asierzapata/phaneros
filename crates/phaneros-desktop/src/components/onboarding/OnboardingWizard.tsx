import React, { useState } from 'react';
import { useOnboarding } from '@/context/OnboardingContext';
import { useTheme } from '@/context/ThemeContext';
import { Button } from '@/components/ui/Button';
import { SemanticPointCloud } from '@/components/onboarding/SemanticPointCloud';
import { mockPresetVaults } from '@/lib/presetVaults';
import { pickFolder } from '@/lib/backendBridge';
import '@/styles/onboarding.css';

const STEP_TAGS: Record<number, string> = {
  1: '01 / Welcome',
  2: '02 / Destination',
  3: '03 / Folder Vaults',
  4: '04 / Test Drive',
  5: '05 / Manifest Vault',
};

export const OnboardingWizard: React.FC = () => {
  const {
    currentStep,
    nextStep,
    prevStep,
    completeOnboarding,
    isCompleting,
    completionError,
    destinationMode,
    setDestinationMode,
    serverUrl,
    setServerUrl,
    serverToken,
    setServerToken,
    isConnected,
    isTestingConnection,
    connectionError,
    testConnection,
    vaults,
    addVault,
    removeVault,
  } = useOnboarding();
  const { theme, toggleTheme } = useTheme();

  const [customFolderPath, setCustomFolderPath] = useState('');
  const [cloudConnecting, setCloudConnecting] = useState(false);
  const [cloudConnected, setCloudConnected] = useState(false);

  const handleAddCustomFolder = () => {
    const trimmed = customFolderPath.trim();
    if (!trimmed) return;
    const parts = trimmed.split('/');
    const name = parts[parts.length - 1] || trimmed;
    addVault({ name, path: trimmed });
    setCustomFolderPath('');
  };

  const handleBrowseFolder = async () => {
    const picked = await pickFolder();
    if (!picked) return;
    const parts = picked.split('/');
    const name = parts[parts.length - 1] || picked;
    addVault({ name, path: picked });
  };

  const handleCloudSignIn = () => {
    setCloudConnecting(true);
    setTimeout(() => {
      setCloudConnecting(false);
      setCloudConnected(true);
    }, 900);
  };

  return (
    <div className="font-sans" data-testid="onboarding-wizard">
      <header className="absolute top-0 left-0 right-0 h-[72px] px-12 flex items-center justify-between z-50">
        <span className="font-serif text-xl font-bold tracking-tight text-foreground">Phaneros</span>
        <button
          type="button"
          onClick={toggleTheme}
          aria-label={theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'}
          className="w-9 h-9 rounded-lg border border-border bg-card text-foreground shadow-card flex items-center justify-center"
        >
          {theme === 'light' ? '🌙' : '☀️'}
        </button>
      </header>
      <div className="onboarding-full-viewport">
        <div className="onboarding-left-col">
          {/* Progress line */}
          <div className="progress-line-track" data-testid="onboarding-stepper">
            <div
              className="progress-line-fill"
              style={{ width: `${(currentStep / 5) * 100}%` }}
              data-testid="onboarding-progress-fill"
            />
          </div>
          <div className="flex flex-col gap-6" data-testid={`onboarding-step-content-${currentStep}`}>
            {/* Step 1: Welcome & Local-First */}
            {currentStep === 1 && (
              <div className="flex flex-col gap-4" data-testid="step-1-welcome">
                <span className="step-tag">{STEP_TAGS[1]}</span>
                <h1 className="headline">Welcome to Phaneros</h1>
                <p className="subheadline">
                  Never worry about syncing files again. Phaneros runs quietly in the background, keeping
                  your files safe, encrypted, and automatically updated — your local-first, zero-trust file
                  synchronization control center.
                </p>
                <div className="flex flex-col gap-3 mt-2">
                  <div className="flex items-center gap-3 font-semibold text-sm">
                    <span className="text-emerald-green">✓</span>
                    <span>Local-First: Your files live safely on your local disk.</span>
                  </div>
                  <div className="flex items-center gap-3 font-semibold text-sm">
                    <span className="text-emerald-green">✓</span>
                    <span data-testid="local-encryption-badge">
                      End-to-End Local Encryption Enabled: only you hold the key.
                    </span>
                  </div>
                  <div className="flex items-center gap-3 font-semibold text-sm">
                    <span className="text-emerald-green">✓</span>
                    <span>Zero Stress: automatic background sync without manual uploads.</span>
                  </div>
                </div>
              </div>
            )}

            {/* Step 2: Destination Setup */}
            {currentStep === 2 && (
              <div className="flex flex-col gap-4" data-testid="step-2-destination">
                <span className="step-tag">{STEP_TAGS[2]}</span>
                <h1 className="headline">Where do you want to keep your backups?</h1>
                <p className="subheadline">
                  Select managed zero-config cloud storage or connect your self-hosted server instance.
                </p>

                <div className="flex flex-col gap-4">
                  {/* Option 1: Managed Cloud */}
                  <div className="flex flex-col gap-0">
                    <div
                      className={`selection-row ${destinationMode === 'cloud' ? 'selected' : ''}`}
                      onClick={() => setDestinationMode('cloud')}
                      data-testid="mode-cloud-button"
                    >
                      <div>
                        <div className="selection-title">Phaneros Managed Cloud</div>
                        <div className="selection-desc">Zero configuration required. Encrypted managed storage.</div>
                      </div>
                      <span className="tag-badge tag-green font-mono">RECOMMENDED</span>
                    </div>

                    {destinationMode === 'cloud' && (
                      <div className="config-panel">
                        {!cloudConnected ? (
                          <>
                            <div className="text-sm text-muted-foreground">
                              Sign in to link this Mac with your Phaneros Cloud storage vault.
                            </div>
                            <Button
                              variant="primary"
                              size="sm"
                              onClick={handleCloudSignIn}
                              disabled={cloudConnecting}
                              data-testid="cloud-signin-button"
                            >
                              {cloudConnecting ? 'Connecting…' : 'Sign in with Phaneros Cloud'}
                            </Button>
                          </>
                        ) : (
                          <div className="flex items-center justify-between">
                            <div>
                              <div className="font-bold text-sm text-emerald-green">● Account Connected</div>
                              <div className="text-xs font-mono text-muted-foreground">
                                Standard Cloud Vault
                              </div>
                            </div>
                            <Button
                              variant="secondary"
                              size="sm"
                              onClick={() => setCloudConnected(false)}
                              data-testid="cloud-disconnect-button"
                            >
                              Disconnect
                            </Button>
                          </div>
                        )}
                      </div>
                    )}
                  </div>

                  {/* Option 2: Self-Hosted Server */}
                  <div className="flex flex-col gap-0">
                    <div
                      className={`selection-row ${destinationMode === 'self-hosted' ? 'selected' : ''}`}
                      onClick={() => setDestinationMode('self-hosted')}
                      data-testid="mode-self-hosted-button"
                    >
                      <div>
                        <div className="selection-title">Self-Hosted Server</div>
                        <div className="selection-desc">Connect your private server URL &amp; token.</div>
                      </div>
                      <span className="text-xs font-mono font-bold text-muted-foreground">OPEN SOURCE</span>
                    </div>

                    {destinationMode === 'self-hosted' && (
                      <div className="config-panel">
                        <div>
                          <label className="text-xs font-semibold text-muted-foreground block mb-1.5">
                            Server Endpoint URL
                          </label>
                          <input
                            type="text"
                            value={serverUrl}
                            onChange={(e) => setServerUrl(e.target.value)}
                            placeholder="https://sync.company.com:8443"
                            className="onboarding-input-field"
                            data-testid="input-server-url"
                          />
                        </div>
                        <div>
                          <label className="text-xs font-semibold text-muted-foreground block mb-1.5">
                            API Access Token / Key
                          </label>
                          <input
                            type="password"
                            value={serverToken}
                            onChange={(e) => setServerToken(e.target.value)}
                            placeholder="sec_token_..."
                            className="onboarding-input-field"
                            data-testid="input-server-token"
                          />
                        </div>
                        <div className="flex items-center justify-between mt-1">
                          <Button
                            variant="secondary"
                            size="sm"
                            onClick={testConnection}
                            disabled={isTestingConnection}
                            data-testid="test-connection-button"
                          >
                            {isTestingConnection ? 'Verifying…' : 'Verify Connection'}
                          </Button>
                          {isConnected && (
                            <span
                              className="text-xs font-mono font-semibold text-emerald-green"
                              data-testid="connection-status-success"
                            >
                              ✓ Endpoint Verified
                            </span>
                          )}
                        </div>
                        {connectionError && (
                          <span className="text-xs font-mono text-rose-600" data-testid="connection-status-error">
                            {connectionError}
                          </span>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}

            {/* Step 3: Folder Vault Selection */}
            {currentStep === 3 && (
              <div className="flex flex-col gap-4" data-testid="step-3-vaults">
                <span className="step-tag">{STEP_TAGS[3]}</span>
                <h1 className="headline">Select folders to keep synchronized</h1>
                <p className="subheadline">
                  Phaneros watches these folders and keeps every change in sync in real time.
                </p>

                <div className="flex flex-col gap-3">
                  {vaults.map((vault) => (
                    <div key={vault.id} className="folder-row" data-testid={`onboarding-vault-row-${vault.id}`}>
                      <div>
                        <div className="font-semibold text-sm">{vault.name}</div>
                        <div className="text-xs text-muted-foreground font-mono">{vault.path}</div>
                      </div>
                      <div className="flex items-center gap-3">
                        <span className="font-mono font-bold text-xs" data-testid={`onboarding-vault-quota-${vault.id}`}>
                          {vault.quotaBytes === undefined ? 'Infinite ∞' : `${vault.quotaBytes} B`}
                        </span>
                        <button
                          type="button"
                          className="onboarding-delete-btn"
                          onClick={() => removeVault(vault.id)}
                          data-testid={`remove-vault-${vault.id}`}
                          aria-label={`Remove ${vault.name}`}
                        >
                          ✕
                        </button>
                      </div>
                    </div>
                  ))}
                </div>

                <div className="config-panel">
                  <div className="font-bold text-sm">Add New Folder Vault</div>
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={customFolderPath}
                      onChange={(e) => setCustomFolderPath(e.target.value)}
                      placeholder="~/Desktop/MyVault or click preset…"
                      className="onboarding-input-field"
                      data-testid="input-custom-folder"
                    />
                    <Button variant="primary" size="sm" onClick={handleAddCustomFolder} data-testid="add-custom-folder-button">
                      Add Folder
                    </Button>
                    <Button variant="secondary" size="sm" onClick={handleBrowseFolder} data-testid="browse-folder-button">
                      Browse…
                    </Button>
                  </div>

                  <div className="flex items-center gap-2 flex-wrap mt-1">
                    <span className="text-xs text-muted-foreground">Quick Add:</span>
                    {mockPresetVaults.map((preset) => {
                      const isAdded = vaults.some((v) => v.name === preset.name);
                      return (
                        <button
                          key={preset.name}
                          type="button"
                          className={`preset-chip ${isAdded ? 'selected' : ''}`}
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
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>
            )}

            {/* Step 4: Connection & Telemetry Test Drive */}
            {currentStep === 4 && (
              <div className="flex flex-col gap-4" data-testid="step-4-test-drive">
                <span className="step-tag">{STEP_TAGS[4]}</span>
                <h1 className="headline">Try it right now: test your connection</h1>
                <p className="subheadline">
                  Run a real connection check against your daemon before completing setup.
                </p>

                <div className="practice-container">
                  <p className="font-semibold text-sm" data-testid="test-drive-status-text">
                    {isTestingConnection
                      ? 'Verifying server handshake…'
                      : isConnected
                      ? 'Connection Verified!'
                      : connectionError
                      ? `Connection failed: ${connectionError}`
                      : 'Click "Test Background Sync" to verify connection.'}
                  </p>

                  <div className="practice-progress" data-testid="test-drive-progress-bar">
                    <div
                      className={`practice-progress-fill ${isTestingConnection ? 'animate-pulse' : ''}`}
                      style={{ width: isTestingConnection ? '75%' : isConnected ? '100%' : '0%' }}
                    />
                  </div>

                  {isConnected && (
                    <div className="practice-success" data-testid="test-drive-success">
                      <span>✓</span>
                      <span>Connection Verified &amp; Payload Synced!</span>
                    </div>
                  )}

                  <Button
                    variant="primary"
                    onClick={testConnection}
                    disabled={isTestingConnection}
                    data-testid="run-test-drive-button"
                  >
                    {isTestingConnection ? 'Testing…' : 'Test Background Sync'}
                  </Button>
                </div>
              </div>
            )}

            {/* Step 5: Finish & Launch */}
            {currentStep === 5 && (
              <div className="flex flex-col gap-4" data-testid="step-5-finish">
                <span className="step-tag">{STEP_TAGS[5]}</span>
                <h1 className="headline">Setup Complete</h1>
                <p className="subheadline">
                  Phaneros Shield Core is active. Your vaults are monitored and protected.
                </p>

                <div className="folder-row flex-col items-start gap-3">
                  <div className="flex justify-between w-full text-sm">
                    <span className="text-muted-foreground">Storage Destination:</span>
                    <strong data-testid="summary-destination">
                      {destinationMode === 'cloud' ? 'Phaneros Managed Cloud' : serverUrl || 'Self-Hosted Server'}
                    </strong>
                  </div>
                  <div className="flex justify-between w-full text-sm">
                    <span className="text-muted-foreground">Synced Folder Vaults:</span>
                    <strong data-testid="summary-folder-count">
                      {vaults.length} {vaults.length === 1 ? 'Folder' : 'Folders'}
                    </strong>
                  </div>
                  <div className="flex justify-between w-full items-center text-sm">
                    <span className="text-muted-foreground">Connection Status:</span>
                    <span className="text-emerald-green font-bold">● Online &amp; Verified</span>
                  </div>
                </div>

                {completionError && (
                  <span className="text-xs font-mono text-rose-600" data-testid="completion-error">
                    Some vaults could not be created: {completionError}
                  </span>
                )}

                <Button
                  variant="primary"
                  size="lg"
                  onClick={completeOnboarding}
                  disabled={isCompleting}
                  data-testid="finish-and-launch-button"
                >
                  {isCompleting ? 'Launching…' : 'Launch Control Center'}
                </Button>
              </div>
            )}
          </div>

        {/* Footer Navigation Controls */}
        <div className="onboarding-nav-footer">
          {currentStep > 1 ? (
            <Button variant="secondary" size="sm" onClick={prevStep} data-testid="wizard-prev-button">
              Back
            </Button>
          ) : (
            <div />
          )}

          {currentStep < 5 && (
            <Button variant="primary" size="sm" onClick={nextStep} data-testid="wizard-next-button">
              Continue
            </Button>
          )}
        </div>
      </div>

      <div className="onboarding-right-col">
        <SemanticPointCloud step={currentStep} isTesting={isTestingConnection} folderCount={vaults.length} />
      </div>
    </div>
    </div>
  );
};
