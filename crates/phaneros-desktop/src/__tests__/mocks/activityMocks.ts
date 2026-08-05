import { ActivitySession } from '@/types';

export const mockRecentActivity: ActivitySession[] = [
  {
    id: 'sess-1',
    driveId: 'sync-protocol',
    timestamp: '2m ago',
    summary: 'Synced 4.2 MB → 1.1 MB (74% smaller)',
  },
  {
    id: 'sess-2',
    driveId: 'phaneros-design',
    timestamp: '10m ago',
    summary: 'Synced 820.0 KB',
  },
  {
    id: 'sess-3',
    driveId: 'app-state',
    timestamp: '1h ago',
    summary: 'Synced 12.4 MB → 3.9 MB (69% smaller)',
  },
];

export const mockEmptyActivity: ActivitySession[] = [];
