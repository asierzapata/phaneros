import { TelemetryMetrics } from '@/types';

export const mockTelemetry: TelemetryMetrics = {
  lastSynced: '2m ago',
  deduplicationRatio: '1.85×',
  compressionRatio: '32%',
  transferSpeed: '4.2 MB/s',
};

export const mockSyncingTelemetry: TelemetryMetrics = {
  lastSynced: 'syncing now',
  deduplicationRatio: '1.85×',
  compressionRatio: '32%',
  transferSpeed: '12.4 MB/s',
};

export const mockIdleTelemetry: TelemetryMetrics = {
  lastSynced: 'Never',
  deduplicationRatio: '1.00×',
  compressionRatio: '0%',
  transferSpeed: '0 B/s',
};
