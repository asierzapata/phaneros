import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { useOnboarding } from '@/context/OnboardingContext';
import { useVault } from '@/context/VaultContext';
import { useTelemetry } from '@/context/TelemetryContext';
import { useView } from '@/context/ViewContext';
import { useTheme } from '@/context/ThemeContext';
import { MainTab, CodeDiff, BinaryMetadataDiff, FileNode, TrayActivityItem } from '@/types';
import React, { useState } from 'react';

// Integrated Application Harness for Tier 4 Real-World Workflows
const PhanerosAppIntegrationHarness: React.FC<{
  initialTextConflicts?: CodeDiff[];
  initialBinaryConflicts?: BinaryMetadataDiff[];
  customFileTree?: FileNode[];
  trayActivity?: TrayActivityItem[];
}> = ({
  initialTextConflicts = [],
  initialBinaryConflicts = [],
  customFileTree = [],
  trayActivity = [],
}) => {
  const { currentStep, isCompleted, destinationMode, setDestinationMode, setServerUrl, setServerToken, setConnected, vaults, addVault: addOnboardingVault, nextStep, completeOnboarding } = useOnboarding();
  const { drives, activeDriveId, selectDrive, addVault: addGlobalVault } = useVault();
  const { metrics, isSyncing, triggerSync } = useTelemetry();
  const { activeTab, setActiveTab } = useView();
  const { theme, toggleTheme } = useTheme();

  const [textConflicts, setTextConflicts] = useState<CodeDiff[]>(initialTextConflicts);
  const [binaryConflicts, setBinaryConflicts] = useState<BinaryMetadataDiff[]>(initialBinaryConflicts);
  const [binaryChoice, setBinaryChoice] = useState<Record<string, 'Keep Local' | 'Keep Store'>>({});

  const totalConflicts = textConflicts.length + binaryConflicts.length;

  // Onboarding view if incomplete
  if (!isCompleted) {
    return (
      <div data-testid="onboarding-flow-container" className="p-8 max-w-2xl mx-auto border rounded-xl">
        <h1 data-testid="onboarding-title">Phaneros Setup Wizard</h1>
        <div data-testid="onboarding-step-indicator">Step {currentStep} of 5</div>

        {currentStep === 1 && (
          <div data-testid="onboarding-step-1">
            <h2>Welcome & Local-First Protection</h2>
            <button data-testid="step1-continue-btn" onClick={nextStep}>
              Get Started
            </button>
          </div>
        )}

        {currentStep === 2 && (
          <div data-testid="onboarding-step-2" className="space-y-4">
            <h2>Destination Setup</h2>
            <div>
              <button
                data-testid="mode-cloud-option"
                onClick={() => setDestinationMode('cloud')}
                className={destinationMode === 'cloud' ? 'font-bold' : ''}
              >
                Phaneros Managed Cloud
              </button>
              <button
                data-testid="mode-selfhosted-option"
                onClick={() => setDestinationMode('self-hosted')}
                className={destinationMode === 'self-hosted' ? 'font-bold' : ''}
              >
                Self-Hosted Endpoint
              </button>
            </div>

            {destinationMode === 'self-hosted' && (
              <div className="space-y-2">
                <input
                  data-testid="selfhosted-url-input"
                  placeholder="https://sync.mycompany.com:8443"
                  onChange={(e) => setServerUrl(e.target.value)}
                />
                <input
                  data-testid="selfhosted-token-input"
                  placeholder="Secret security token..."
                  onChange={(e) => setServerToken(e.target.value)}
                />
                <button
                  data-testid="test-connection-btn"
                  onClick={() => setConnected(true)}
                >
                  Test Connection
                </button>
              </div>
            )}

            <button data-testid="step2-continue-btn" onClick={nextStep}>
              Continue to Vaults
            </button>
          </div>
        )}

        {currentStep === 3 && (
          <div data-testid="onboarding-step-3" className="space-y-4">
            <h2>Folder Vault Selection</h2>
            <button
              data-testid="add-documents-vault-btn"
              onClick={() => {
                const vault = { name: 'Documents', path: '~/Documents' };
                addOnboardingVault(vault);
                addGlobalVault(vault);
              }}
            >
              Add ~/Documents Preset
            </button>
            <ul data-testid="selected-vaults-list">
              {vaults.map((v) => (
                <li key={v.id} data-testid={`onboarding-vault-item-${v.name}`}>
                  {v.name} - {v.quotaBytes === undefined ? 'Infinite ∞' : `${v.quotaBytes} B`}
                </li>
              ))}
            </ul>
            <button data-testid="step3-continue-btn" onClick={nextStep} disabled={vaults.length === 0}>
              Continue to Test Drive
            </button>
          </div>
        )}

        {currentStep === 4 && (
          <div data-testid="onboarding-step-4">
            <h2>Connection & Telemetry Test Drive</h2>
            <p>Verification successful!</p>
            <button data-testid="step4-continue-btn" onClick={nextStep}>
              Proceed to Finish
            </button>
          </div>
        )}

        {currentStep === 5 && (
          <div data-testid="onboarding-step-5">
            <h2>Setup Complete & Shield Core</h2>
            <button data-testid="complete-setup-btn" onClick={completeOnboarding}>
              Open Main Desktop Control Center
            </button>
          </div>
        )}
      </div>
    );
  }

  // Main Desktop Control Center
  return (
    <div data-testid="app-main-window" className={`min-h-screen ${theme}`}>
      {/* Top Header Bar */}
      <header className="transparent-top-bar flex justify-between items-center p-4 border-b">
        <div className="w-[180px]">
          <span className="brand-wordmark font-serif">PHANEROS</span>
        </div>

        <nav role="tablist" className="hig-segmented-control flex space-x-2">
          {(['dashboard', 'drives', 'conflicts', 'activity', 'settings'] as MainTab[]).map((tab) => (
            <button
              key={tab}
              role="tab"
              aria-selected={activeTab === tab}
              onClick={() => setActiveTab(tab)}
              data-testid={`nav-tab-${tab}`}
              className={`px-3 py-1.5 rounded ${activeTab === tab ? 'bg-primary text-white font-bold' : ''}`}
            >
              {tab.toUpperCase()}
              {tab === 'conflicts' && totalConflicts > 0 && (
                <span data-testid="header-conflict-count" className="ml-1 bg-red-500 text-white rounded-full px-1.5 text-xs">
                  {totalConflicts}
                </span>
              )}
            </button>
          ))}
        </nav>

        <div className="w-[180px] flex justify-end">
          <button data-testid="header-theme-toggle" onClick={toggleTheme}>
            Toggle Theme
          </button>
        </div>
      </header>

      {/* Main Tab Content */}
      <main className="p-6">
        {activeTab === 'dashboard' && (
          <div data-testid="view-dashboard" className="space-y-6">
            <section
              data-testid="hero-health-banner"
              className={`p-4 rounded-lg border ${totalConflicts > 0 ? 'bg-amber-500/10 border-amber-500' : 'bg-emerald-500/10 border-emerald-500'}`}
            >
              <h2 data-testid="hero-status-heading">
                {totalConflicts > 0 ? `${totalConflicts} Conflicts Pending` : 'All Systems Synced & Protected'}
              </h2>
              <button data-testid="hero-sync-btn" onClick={triggerSync} disabled={isSyncing}>
                {isSyncing ? 'Syncing...' : 'Sync Now'}
              </button>
            </section>

            <section data-testid="telemetry-panel" className="grid grid-cols-4 gap-4 p-4 border rounded">
              <div>Last Synced: <span data-testid="tel-last-synced">{metrics.lastSynced}</span></div>
              <div>Dedup Ratio: <span data-testid="tel-dedup">{metrics.deduplicationRatio}</span></div>
              <div>Compression: <span data-testid="tel-compression">{metrics.compressionRatio}</span></div>
              <div>Speed: <span data-testid="tel-speed">{metrics.transferSpeed}</span></div>
            </section>

            <section data-testid="drives-grid-panel" className="grid grid-cols-2 gap-4">
              {drives.map((drive) => (
                <div key={drive.id} data-testid={`main-drive-card-${drive.name}`} className="p-4 border rounded">
                  <h3>{drive.name}</h3>
                  <p>
                    Capacity:{' '}
                    <span data-testid={`main-drive-quota-${drive.name}`}>
                      {drive.quotaBytes === undefined ? 'Infinite ∞' : `${drive.quotaBytes} B`}
                    </span>
                  </p>
                  <p data-testid={`main-drive-status-${drive.name}`}>Status: {drive.status}</p>
                </div>
              ))}
            </section>
          </div>
        )}

        {activeTab === 'drives' && (
          <div data-testid="view-drives" className="flex gap-6">
            <aside data-testid="drive-browser-selector" className="w-64 border-r pr-4">
              <h3>Configured Drives</h3>
              <ul>
                {drives.map((d) => (
                  <li
                    key={d.id}
                    data-testid={`drive-selector-item-${d.id}`}
                    onClick={() => selectDrive(d.id)}
                    className={`cursor-pointer p-2 ${activeDriveId === d.id ? 'font-bold bg-primary/10' : ''}`}
                  >
                    {d.name}
                  </li>
                ))}
              </ul>
            </aside>
            <section data-testid="file-explorer-tree" className="flex-1">
              <h3>File Hierarchy</h3>
              {customFileTree.map((node) => (
                <div key={node.id} data-testid={`explorer-node-${node.id}`} className="py-1">
                  <span>{node.name}</span>
                  {node.badge && <span data-testid={`explorer-badge-${node.id}`} className="ml-2 px-1 text-xs bg-slate-200">{node.badge}</span>}
                </div>
              ))}
            </section>
          </div>
        )}

        {activeTab === 'conflicts' && (
          <div data-testid="view-conflicts" className="space-y-6">
            <h2>Conflict Resolution Workspace</h2>
            {totalConflicts === 0 ? (
              <div data-testid="no-conflicts-notice">No active conflicts. All vaults fully synchronized!</div>
            ) : (
              <div>
                {textConflicts.map((tc) => (
                  <div key={tc.filename} data-testid={`text-conflict-card-${tc.filename}`} className="border p-4 mb-4">
                    <h3>{tc.filename} (Code Diff)</h3>
                    <button
                      data-testid={`apply-text-resolve-${tc.filename}`}
                      onClick={() => setTextConflicts((prev) => prev.filter((item) => item.filename !== tc.filename))}
                      className="px-3 py-1 bg-emerald-600 text-white rounded"
                    >
                      Resolve & Sync
                    </button>
                  </div>
                ))}

                {binaryConflicts.map((bc) => {
                  const choice = binaryChoice[bc.filename] || bc.recommendedAction;
                  return (
                    <div key={bc.filename} data-testid={`binary-conflict-card-${bc.filename}`} className="border p-4 mb-4">
                      <h3>{bc.filename} (Opaque Binary Matrix)</h3>
                      <p data-testid={`binary-recommended-${bc.filename}`}>Recommended: {bc.recommendedAction}</p>
                      <div className="grid grid-cols-2 gap-4 my-2">
                        <div>Local Size: {bc.local.size} | Hash: {bc.local.hash}</div>
                        <div>Store Size: {bc.store.size} | Hash: {bc.store.hash}</div>
                      </div>
                      <div className="flex space-x-2">
                        <button
                          data-testid={`select-keep-local-${bc.filename}`}
                          onClick={() => setBinaryChoice((prev) => ({ ...prev, [bc.filename]: 'Keep Local' }))}
                        >
                          Keep Local
                        </button>
                        <button
                          data-testid={`select-keep-store-${bc.filename}`}
                          onClick={() => setBinaryChoice((prev) => ({ ...prev, [bc.filename]: 'Keep Store' }))}
                        >
                          Keep Store
                        </button>
                        <button
                          data-testid={`apply-binary-resolve-${bc.filename}`}
                          onClick={() => setBinaryConflicts((prev) => prev.filter((item) => item.filename !== bc.filename))}
                          className="px-3 py-1 bg-emerald-600 text-white rounded"
                        >
                          Resolve Choice ({choice})
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}

        {activeTab === 'activity' && <div data-testid="view-activity">Activity Feed</div>}

        {activeTab === 'settings' && <div data-testid="view-settings">Application Settings</div>}
      </main>

      {/* 380px System Tray Popup Mock Component */}
      <div data-testid="compact-tray-popup" className="fixed bottom-4 right-4 w-[380px] p-3 border bg-card shadow-2xl rounded-lg">
        <div className="flex justify-between items-center mb-2">
          <span className="font-bold text-xs">System Tray</span>
          <button data-testid="tray-nav-settings" onClick={() => setActiveTab('settings')}>
            ⚙ Settings
          </button>
        </div>
        <div data-testid="tray-progress-ring">Syncing 68%</div>
        <div data-testid="tray-activity-list">
          {trayActivity.map((act) => (
            <div key={act.id} data-testid={`tray-act-${act.id}`} className="text-xs">
              {act.name} <span data-testid={`tray-act-badge-${act.id}`}>{act.ext}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

describe('Tier 4 Real-World Application Workflows (RW-01 to RW-05)', () => {
  it('RW-01: Full Onboarding to Main Dashboard Flow with Infinite ∞ Quota Verification', async () => {
    const user = userEvent.setup();
    render(<PhanerosAppIntegrationHarness />);

    // Step 1: Welcome Screen
    expect(screen.getByTestId('onboarding-title')).toHaveTextContent('Phaneros Setup Wizard');
    await user.click(screen.getByTestId('step1-continue-btn'));

    // Step 2: Destination Setup -> Select Self-Hosted
    expect(screen.getByTestId('onboarding-step-2')).toBeInTheDocument();
    await user.click(screen.getByTestId('mode-selfhosted-option'));
    await user.type(screen.getByTestId('selfhosted-url-input'), 'https://sync.phaneros.io:8443');
    await user.type(screen.getByTestId('selfhosted-token-input'), 'sec_token_987654321');
    await user.click(screen.getByTestId('test-connection-btn'));
    await user.click(screen.getByTestId('step2-continue-btn'));

    // Step 3: Vault Selection -> Add Preset
    expect(screen.getByTestId('onboarding-step-3')).toBeInTheDocument();
    await user.click(screen.getByTestId('add-documents-vault-btn'));
    expect(screen.getByTestId('onboarding-vault-item-Documents')).toHaveTextContent('Documents - Infinite ∞');
    await user.click(screen.getByTestId('step3-continue-btn'));

    // Step 4: Telemetry Test Drive
    expect(screen.getByTestId('onboarding-step-4')).toBeInTheDocument();
    await user.click(screen.getByTestId('step4-continue-btn'));

    // Step 5: Setup Complete -> Transition to Main Control Center
    expect(screen.getByTestId('onboarding-step-5')).toBeInTheDocument();
    await user.click(screen.getByTestId('complete-setup-btn'));

    // Verify Main Desktop Control Center Dashboard renders with newly configured vault and Infinite quota rule
    expect(screen.getByTestId('app-main-window')).toBeInTheDocument();
    expect(screen.getByTestId('hero-status-heading')).toHaveTextContent('All Systems Synced & Protected');
    expect(screen.getByTestId('main-drive-quota-Documents')).toHaveTextContent('Infinite ∞');
  });

  it('RW-02: Code Conflict Detection, Diff Inspection & Resolution Workflow', async () => {
    const user = userEvent.setup();
    const textConflict: CodeDiff = {
      filename: 'vault.rs',
      path: '~/Developer/Projects/src/vault.rs',
      linesAdded: 2,
      linesRemoved: 1,
      chunks: [
        {
          oldStart: 1,
          newStart: 1,
          lines: [
            { type: 'delete', text: '- pub quota: u64' },
            { type: 'add', text: '+ pub quota: Option<u64>' },
          ],
        },
      ],
    };

    render(
      <PhanerosAppIntegrationHarness
        initialTextConflicts={[textConflict]}
      />,
      {
        providerProps: {
          onboardingProps: { initialState: { isCompleted: true } },
        },
      }
    );

    // Verify header conflict badge indicates 1 conflict
    expect(screen.getByTestId('header-conflict-count')).toHaveTextContent('1');

    // Click Conflicts tab
    await user.click(screen.getByTestId('nav-tab-conflicts'));
    expect(screen.getByTestId('view-conflicts')).toBeInTheDocument();
    expect(screen.getByTestId('text-conflict-card-vault.rs')).toBeInTheDocument();

    // Resolve conflict
    await user.click(screen.getByTestId('apply-text-resolve-vault.rs'));

    // Verify all conflicts resolved banner and header badge cleared
    expect(screen.getByTestId('no-conflicts-notice')).toBeInTheDocument();
    expect(screen.queryByTestId('header-conflict-count')).not.toBeInTheDocument();
  });

  it('RW-03: Opaque Binary Conflict Resolution with Custom Choice Selection', async () => {
    const user = userEvent.setup();
    const binaryConflict: BinaryMetadataDiff = {
      filename: 'state.sqlite',
      path: '~/Documents/state.sqlite',
      isBinary: true,
      local: { size: '14.2 MB', modified: '2026-08-04 22:00', hash: 'local_hash_123' },
      store: { size: '15.8 MB', modified: '2026-08-04 22:15', hash: 'store_hash_456' },
      recommendedAction: 'Keep Local',
    };

    render(
      <PhanerosAppIntegrationHarness
        initialBinaryConflicts={[binaryConflict]}
      />,
      {
        providerProps: {
          onboardingProps: { initialState: { isCompleted: true } },
        },
      }
    );

    // Navigate to Conflicts tab
    await user.click(screen.getByTestId('nav-tab-conflicts'));
    expect(screen.getByTestId('binary-conflict-card-state.sqlite')).toBeInTheDocument();
    expect(screen.getByTestId('binary-recommended-state.sqlite')).toHaveTextContent('Recommended: Keep Local');

    // Override recommendation to Keep Store
    await user.click(screen.getByTestId('select-keep-store-state.sqlite'));

    // Apply choice
    await user.click(screen.getByTestId('apply-binary-resolve-state.sqlite'));

    // Verify conflicts cleared
    expect(screen.getByTestId('no-conflicts-notice')).toHaveTextContent('All vaults fully synchronized!');
  });

  it('RW-04: File Explorer Hierarchy Navigation & Split Drive Switching', async () => {
    const user = userEvent.setup();
    const fileTreeNodes: FileNode[] = [
      {
        id: 'node-rs-1',
        name: 'sync-protocol.rs',
        ext: 'rs',
        isDir: false,
        badge: 'RS',
      },
    ];

    render(
      <PhanerosAppIntegrationHarness customFileTree={fileTreeNodes} />,
      {
        providerProps: {
          onboardingProps: { initialState: { isCompleted: true } },
        },
      }
    );

    // Navigate to Drives & Files view
    await user.click(screen.getByTestId('nav-tab-drives'));
    expect(screen.getByTestId('view-drives')).toBeInTheDocument();

    // Select code-vault drive
    const driveItem = screen.getByTestId('drive-selector-item-vault-code');
    await user.click(driveItem);
    expect(driveItem).toHaveClass('bg-primary/10');

    // Verify file tree renders node with RS badge
    expect(screen.getByTestId('explorer-node-node-rs-1')).toHaveTextContent('sync-protocol.rs');
    expect(screen.getByTestId('explorer-badge-node-rs-1')).toHaveTextContent('RS');
  });

  it('RW-05: System Tray Popup Monitoring & Direct Settings Navigation', async () => {
    const user = userEvent.setup();
    const trayItems: TrayActivityItem[] = [
      {
        id: 'act-tray-1',
        name: 'sync-protocol',
        ext: 'RS',
        timestamp: '1m ago',
        action: 'synced',
      },
    ];

    render(
      <PhanerosAppIntegrationHarness trayActivity={trayItems} />,
      {
        providerProps: {
          onboardingProps: { initialState: { isCompleted: true } },
        },
      }
    );

    // Verify 380px compact system tray rendered
    expect(screen.getByTestId('compact-tray-popup')).toBeInTheDocument();
    expect(screen.getByTestId('tray-progress-ring')).toHaveTextContent('Syncing 68%');
    expect(screen.getByTestId('tray-act-badge-act-tray-1')).toHaveTextContent('RS');

    // Click settings button in tray popup
    await user.click(screen.getByTestId('tray-nav-settings'));

    // Verify Main Control Center transitions to Settings view
    expect(screen.getByTestId('view-settings')).toBeInTheDocument();
  });
});
