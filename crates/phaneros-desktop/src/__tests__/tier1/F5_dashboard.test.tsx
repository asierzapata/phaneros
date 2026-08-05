import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { Dashboard } from '@/components/main/Dashboard';
import { mockDrives } from '../mocks/vaultMocks';
import { mockTelemetry } from '../mocks/telemetryMocks';

describe('F5_DASH: System Dashboard Module & Telemetry', () => {
  it('F5-T1-01: should render Hero health status card with emerald green checkmark ✓', () => {
    render(<Dashboard />);
    const heroCard = screen.getByTestId('hero-health-card');
    expect(heroCard).toBeInTheDocument();
    expect(screen.getByText('✓')).toBeInTheDocument();
    expect(screen.getByText(/System Health Normal/i)).toBeInTheDocument();
  });

  it('F5-T1-02: should render manual Sync Now button and trigger sync action', async () => {
    const user = userEvent.setup();
    render(<Dashboard />);

    const syncButton = screen.getByTestId('sync-now-button');
    expect(syncButton).toHaveTextContent('Sync Now');

    await user.click(syncButton);
    expect(syncButton).toHaveTextContent('Syncing...');
  });

  it('F5-T1-03: should render 4 telemetry metrics grid (last synced, deduplication ratio, compression ratio, transfer speed)', () => {
    render(<Dashboard />, {
      providerProps: {
        telemetryProps: { initialMetrics: mockTelemetry },
      },
    });

    const metricsGrid = screen.getByTestId('telemetry-metrics-grid');
    expect(metricsGrid).toBeInTheDocument();

    expect(screen.getByText(mockTelemetry.lastSynced)).toBeInTheDocument();
    expect(screen.getByText(mockTelemetry.deduplicationRatio)).toBeInTheDocument();
    expect(screen.getByText(mockTelemetry.compressionRatio)).toBeInTheDocument();
    expect(screen.getByText(mockTelemetry.transferSpeed)).toBeInTheDocument();
  });

  it('F5-T1-04: should render storage drive cards displaying Infinite ∞ quota', () => {
    render(<Dashboard />, {
      providerProps: {
        vaultProps: { initialDrives: mockDrives },
      },
    });

    mockDrives.forEach((drive) => {
      const quotaElement = screen.getByTestId(`quota-display-${drive.id}`);
      expect(quotaElement).toHaveTextContent('Infinite ∞');
    });
  });

  it('F5-T1-[#05]: should display active drive status pill (synced, syncing, conflict, paused)', () => {
    render(<Dashboard />, {
      providerProps: {
        vaultProps: { initialDrives: mockDrives },
      },
    });

    expect(screen.getByTestId(`drive-card-${mockDrives[0].id}`)).toHaveTextContent('synced');
  });
});
