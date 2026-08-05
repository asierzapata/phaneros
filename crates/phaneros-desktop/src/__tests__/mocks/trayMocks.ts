import { DriveVault, TrayActivityItem } from '@/types';

export interface TrayHealthState {
  status: 'synced' | 'syncing' | 'conflict';
  label: string;
  progressPercent?: number;
}

export const mockTrayHealthSynced: TrayHealthState = {
  status: 'synced',
  label: 'Up to Date',
};

export const mockTrayHealthSyncing: TrayHealthState = {
  status: 'syncing',
  label: 'Syncing 68%',
  progressPercent: 68,
};

export const mockTrayHealthConflict: TrayHealthState = {
  status: 'conflict',
  label: '1 Conflict Pending',
};

export const mockTrayDriveStack: DriveVault[] = [
  {
    id: 'vault-docs',
    name: 'Documents Vault',
    path: '~/Documents/PhanerosSync',
    status: 'synced',
    usedBytes: 45957380096,
    quotaBytes: undefined, // Infinite (∞) capacity
    fileCount: 1240,
  },
  {
    id: 'vault-dev',
    name: 'Developer Vault',
    path: '~/Developer/Projects',
    status: 'synced',
    usedBytes: 15032385536,
    quotaBytes: undefined, // Infinite (∞) capacity
    fileCount: 890,
  },
];

export const mockTrayRecentActivity: TrayActivityItem[] = [
  {
    id: 'act-1',
    name: 'sync-protocol',
    ext: 'RS',
    timestamp: '2m ago',
    action: 'synced',
  },
  {
    id: 'act-2',
    name: 'phaneros-design',
    ext: 'MD',
    timestamp: '10m ago',
    action: 'modified',
  },
  {
    id: 'act-3',
    name: 'app-state',
    ext: 'DB',
    timestamp: '1h ago',
    action: 'synced',
  },
];

export const mockTrayEmptyActivity: TrayActivityItem[] = [];
