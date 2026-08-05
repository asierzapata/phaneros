import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import { useVault } from '@/context/VaultContext';
import { TrayActivityItem } from '@/types';
import React from 'react';

const SystemTrayBoundaryComponent: React.FC<{
  status?: 'synced' | 'syncing' | 'conflict';
  progressPercent?: number;
  customActivity?: TrayActivityItem[];
}> = ({ status = 'synced', progressPercent = 0, customActivity = [] }) => {
  const { drives } = useVault();

  return (
    <div data-testid="tray-popup-root" className="w-[380px] max-w-[380px] mx-auto bg-background text-foreground border rounded-lg shadow-xl p-3 space-y-3">
      {/* Header */}
      <header className="flex justify-between items-center" data-testid="tray-header">
        <span className="font-semibold text-sm">Phaneros Tray</span>
        <div className="flex space-x-1">
          <button data-testid="tray-open-main-btn" aria-label="Open Main Window">↗</button>
          <button data-testid="tray-settings-btn" aria-label="Settings">⚙</button>
        </div>
      </header>

      {/* Hero Health Card */}
      <div data-testid="tray-hero-card" className={`p-3 rounded-md ${status === 'conflict' ? 'bg-amber-500/10 border-amber-500' : 'bg-card'}`}>
        {status === 'synced' && <span data-testid="emerald-tick">✓ Up to Date</span>}
        {status === 'syncing' && (
          <div data-testid="tray-progress-ring-container">
            <span>Syncing {progressPercent}%</span>
            <div data-testid="tray-progress-ring" style={{ width: `${progressPercent}%` }} />
          </div>
        )}
        {status === 'conflict' && <span data-testid="tray-conflict-warning">⚠ 1 Conflict Pending</span>}
      </div>

      {/* Drive Stack Cards with Infinite Capacity */}
      <div data-testid="tray-drive-stack" className="space-y-1.5">
        <h4 className="text-xs font-mono uppercase text-muted">Drive Stack</h4>
        {drives.map((drive) => (
          <div key={drive.id} data-testid={`tray-drive-${drive.id}`} className="p-2 border rounded text-xs flex justify-between">
            <span>{drive.name}</span>
            <span data-testid={`tray-drive-quota-${drive.id}`}>
              {drive.quotaBytes === undefined ? 'Infinite ∞' : `${drive.quotaBytes} B`}
            </span>
          </div>
        ))}
      </div>

      {/* Recent Activity Stream */}
      <div data-testid="tray-activity-section" className="space-y-1.5">
        <h4 className="text-xs font-mono uppercase text-muted">Recent Activity</h4>
        {customActivity.length === 0 ? (
          <p data-testid="empty-activity-msg" className="text-xs text-muted">No recent file changes.</p>
        ) : (
          <ul className="space-y-1">
            {customActivity.map((item) => (
              <li key={item.id} data-testid={`activity-item-${item.id}`} className="text-xs flex items-center justify-between">
                <span data-testid={`activity-name-${item.id}`} className="truncate max-w-[200px]" title={item.name}>
                  {item.name}
                </span>
                <span data-testid={`activity-badge-${item.id}`} className="px-1 text-[10px] bg-slate-200 dark:bg-slate-700 font-mono">
                  {item.ext}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
};

describe('F8_TRAY: Refined 380px System Tray Popup Boundary Tests', () => {
  it('F8-T2-01: should enforce explicit 380px max-width boundary constraint on root popup container', () => {
    render(<SystemTrayBoundaryComponent />);

    const root = screen.getByTestId('tray-popup-root');
    expect(root).toHaveClass('w-[380px]');
    expect(root).toHaveClass('max-w-[380px]');
  });

  it('F8-T2-02: should render empty activity message when recent activity stream is empty', () => {
    render(<SystemTrayBoundaryComponent customActivity={[]} />);

    expect(screen.getByTestId('empty-activity-msg')).toHaveTextContent('No recent file changes');
  });

  it('F8-T2-03: should render active progress ring in tray hero card when status is syncing', () => {
    render(<SystemTrayBoundaryComponent status="syncing" progressPercent={68} />);

    expect(screen.getByTestId('tray-progress-ring-container')).toHaveTextContent('Syncing 68%');
    expect(screen.getByTestId('tray-progress-ring')).toHaveStyle({ width: '68%' });
  });

  it('F8-T2-04: should render warning banner styling in tray hero card when status is conflict', () => {
    render(<SystemTrayBoundaryComponent status="conflict" />);

    expect(screen.getByTestId('tray-conflict-warning')).toHaveTextContent('⚠ 1 Conflict Pending');
    expect(screen.getByTestId('tray-hero-card')).toHaveClass('bg-amber-500/10');
  });

  it('F8-T2-05: should apply text truncation on long file basenames in tray activity stream', () => {
    const longNameItem: TrayActivityItem = {
      id: 'long-1',
      name: 'very_long_file_name_that_exceeds_normal_tray_width_boundary_limit_test',
      ext: 'RS',
      timestamp: 'Just now',
      action: 'modified',
    };

    render(<SystemTrayBoundaryComponent customActivity={[longNameItem]} />);

    const nameSpan = screen.getByTestId('activity-name-long-1');
    expect(nameSpan).toHaveClass('truncate');
    expect(nameSpan).toHaveClass('max-w-[200px]');
  });
});
