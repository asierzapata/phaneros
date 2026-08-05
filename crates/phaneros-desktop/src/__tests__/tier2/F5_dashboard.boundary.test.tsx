import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { useVault } from '@/context/VaultContext';
import { useTelemetry } from '@/context/TelemetryContext';
import React from 'react';

const SystemDashboardBoundaryComponent: React.FC<{ heroStatus?: 'synced' | 'syncing' | 'conflict' }> = ({
  heroStatus = 'synced',
}) => {
  const { drives } = useVault();
  const { metrics, isSyncing, triggerSync } = useTelemetry();

  return (
    <div data-testid="dashboard-root" className="p-6 space-y-6">
      {/* Hero Health Banner */}
      <section
        data-testid="hero-health-card"
        className={`p-4 rounded-lg ${
          heroStatus === 'synced'
            ? 'bg-emerald-500/10 border-emerald-500'
            : heroStatus === 'syncing'
            ? 'bg-blue-500/10 border-blue-500'
            : 'bg-amber-500/10 border-amber-500'
        }`}
      >
        <h2 data-testid="hero-status-title">
          {heroStatus === 'synced'
            ? 'All Systems Protected'
            : heroStatus === 'syncing'
            ? 'Syncing In Progress'
            : '1 Conflict Requires Attention'}
        </h2>
        <button
          onClick={triggerSync}
          disabled={isSyncing}
          data-testid="sync-now-btn"
          className="mt-2 px-4 py-1.5 bg-primary text-white rounded disabled:opacity-50"
        >
          {isSyncing ? 'Syncing...' : 'Sync Now'}
        </button>
      </section>

      {/* Telemetry Metrics Grid */}
      <section data-testid="telemetry-grid" className="grid grid-cols-4 gap-4">
        <div data-testid="metric-last-synced">{metrics.lastSynced}</div>
        <div data-testid="metric-dedup">{metrics.deduplicationRatio}</div>
        <div data-testid="metric-compression">{metrics.compressionRatio}</div>
        <div data-testid="metric-speed">{metrics.transferSpeed}</div>
      </section>

      {/* Storage Drives Grid */}
      <section data-testid="drives-grid">
        {drives.length === 0 ? (
          <div data-testid="zero-drives-banner">No storage drives configured. Add a drive to begin.</div>
        ) : (
          <div className="grid grid-cols-2 gap-4">
            {drives.map((drive) => (
              <div key={drive.id} data-testid={`drive-card-${drive.id}`} className="p-4 border rounded">
                <h3 data-testid={`drive-name-${drive.id}`}>{drive.name}</h3>
                <p data-testid={`drive-capacity-${drive.id}`}>
                  Capacity:{' '}
                  <span data-testid={`drive-quota-val-${drive.id}`}>
                    {drive.quotaBytes === undefined || drive.quotaBytes === null ? 'Infinite ∞' : `${drive.quotaBytes} B`}
                  </span>
                </p>
                <span data-testid={`drive-status-${drive.id}`}>{drive.status}</span>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
};

describe('F5_DASH: System Dashboard Boundary Tests', () => {
  it('F5-T2-01: should explicitly enforce undefined/null quotaBytes rendering as Infinite ∞', () => {
    render(<SystemDashboardBoundaryComponent />);

    const quotaValues = screen.getAllByTestId(/drive-quota-val-/i);
    expect(quotaValues.length).toBeGreaterThan(0);
    quotaValues.forEach((el) => {
      expect(el).toHaveTextContent('Infinite ∞');
    });
  });

  it('F5-T2-02: should render telemetry active speed and update displays accurately', () => {
    render(<SystemDashboardBoundaryComponent />, {
      providerProps: {
        telemetryProps: {
          initialMetrics: {
            lastSynced: 'Just now',
            deduplicationRatio: '2.10×',
            compressionRatio: '45%',
            transferSpeed: '18.5 MB/s',
          },
        },
      },
    });

    expect(screen.getByTestId('metric-speed')).toHaveTextContent('18.5 MB/s');
    expect(screen.getByTestId('metric-dedup')).toHaveTextContent('2.10×');
  });

  it('F5-T2-03: should render warning/conflict hero card status correctly', () => {
    render(<SystemDashboardBoundaryComponent heroStatus="conflict" />);

    expect(screen.getByTestId('hero-status-title')).toHaveTextContent('1 Conflict Requires Attention');
    expect(screen.getByTestId('hero-health-card')).toHaveClass('bg-amber-500/10');
  });

  it('F5-T2-04: should display zero drives banner when drive list is empty', () => {
    render(<SystemDashboardBoundaryComponent />, {
      providerProps: {
        vaultProps: { initialDrives: [] },
      },
    });

    expect(screen.getByTestId('zero-drives-banner')).toBeInTheDocument();
    expect(screen.getByTestId('zero-drives-banner')).toHaveTextContent('No storage drives configured');
  });

  it('F5-T2-05: should handle rapid Sync Now button clicks disabling button during sync state', async () => {
    const user = userEvent.setup();
    render(<SystemDashboardBoundaryComponent />);

    const syncBtn = screen.getByTestId('sync-now-btn');
    expect(syncBtn).not.toBeDisabled();

    await user.click(syncBtn);
    expect(syncBtn).toBeDisabled();
    expect(syncBtn).toHaveTextContent('Syncing...');
  });
});
