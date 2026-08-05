import React from 'react';
import { useTelemetry } from '@/context/TelemetryContext';
import { useVault } from '@/context/VaultContext';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';

export const Dashboard: React.FC = () => {
  const { metrics, isSyncing, triggerSync } = useTelemetry();
  const { drives } = useVault();

  return (
    <div className="flex flex-col gap-6 p-6 max-w-6xl mx-auto font-sans" data-testid="system-dashboard">
      {/* Hero Health Status Card */}
      <Card elevation="medium" className="flex items-center justify-between p-6 bg-card" data-testid="hero-health-card">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-full bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 flex items-center justify-center text-2xl font-bold">
            ✓
          </div>
          <div>
            <h2 className="text-xl font-serif font-bold text-foreground">System Health Normal</h2>
            <p className="text-sm text-muted-foreground">All connected vaults are encrypted and fully synchronized.</p>
          </div>
        </div>
        <Button
          variant="primary"
          onClick={triggerSync}
          disabled={isSyncing}
          data-testid="sync-now-button"
        >
          {isSyncing ? 'Syncing...' : 'Sync Now'}
        </Button>
      </Card>

      {/* 4 Telemetry Metrics Grid */}
      <div className="grid grid-cols-4 gap-4" data-testid="telemetry-metrics-grid">
        <Card elevation="low" className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground uppercase tracking-wider">Last Synced</span>
          <span className="text-lg font-mono font-bold text-foreground">{metrics.lastSynced}</span>
        </Card>
        <Card elevation="low" className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground uppercase tracking-wider">Deduplication Ratio</span>
          <span className="text-lg font-mono font-bold text-foreground">{metrics.deduplicationRatio}</span>
        </Card>
        <Card elevation="low" className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground uppercase tracking-wider">Compression Ratio</span>
          <span className="text-lg font-mono font-bold text-foreground">{metrics.compressionRatio}</span>
        </Card>
        <Card elevation="low" className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground uppercase tracking-wider">Transfer Speed</span>
          <span className="text-lg font-mono font-bold text-foreground">{metrics.transferSpeed}</span>
        </Card>
      </div>

      {/* Configured Storage Drive Cards */}
      <div className="flex flex-col gap-3" data-testid="storage-drives-section">
        <h3 className="text-base font-serif font-bold text-foreground">Configured Storage Drives</h3>
        <div className="grid grid-cols-2 gap-4">
          {drives.map((drive) => {
            const quotaDisplay = drive.quotaBytes === undefined ? 'Infinite ∞' : `${drive.quotaBytes} B`;
            return (
              <Card key={drive.id} elevation="low" className="flex flex-col gap-2 p-4" data-testid={`drive-card-${drive.id}`}>
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-foreground">{drive.name}</span>
                  <Badge variant={drive.status}>{drive.status}</Badge>
                </div>
                <div className="text-xs font-mono text-muted-foreground">{drive.path}</div>
                <div className="flex justify-between items-center text-xs mt-2 pt-2 border-t border-border">
                  <span className="text-muted-foreground">Capacity / Quota:</span>
                  <span className="font-mono font-bold text-foreground" data-testid={`quota-display-${drive.id}`}>
                    {quotaDisplay}
                  </span>
                </div>
              </Card>
            );
          })}
        </div>
      </div>
    </div>
  );
};
