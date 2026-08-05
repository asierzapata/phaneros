import { DriveVault } from '@/types';

export const mockDrives: DriveVault[] = [
  {
    id: 'vault-default',
    name: 'default',
    path: '~/Documents/PhanerosSync',
    status: 'synced',
    usedBytes: 45957380096, // 42.8 GB
    quotaBytes: undefined, // Infinite (∞) quota rule
    fileCount: 1240,
  },
  {
    id: 'vault-code',
    name: 'code-vault',
    path: '~/Developer/Projects',
    status: 'synced',
    usedBytes: 15032385536, // 14.0 GB
    quotaBytes: undefined, // Infinite (∞) quota rule
    fileCount: 890,
  },
];

export const mockSyncedDrive: DriveVault = {
  id: 'vault-synced-1',
  name: 'documents-vault',
  path: '~/Documents/PhanerosSync',
  status: 'synced',
  usedBytes: 45957380096,
  quotaBytes: undefined, // Infinite (∞) quota rule
  fileCount: 1240,
};

export const mockSyncingDrive: DriveVault = {
  id: 'vault-syncing-1',
  name: 'pictures-vault',
  path: '~/Pictures/Family',
  status: 'syncing',
  usedBytes: 12884901888,
  quotaBytes: undefined, // Infinite (∞) quota rule
  fileCount: 450,
};

export const mockConflictDrive: DriveVault = {
  id: 'vault-conflict-1',
  name: 'work-vault',
  path: '~/Desktop/Work',
  status: 'conflict',
  usedBytes: 8589934592,
  quotaBytes: undefined, // Infinite (∞) quota rule
  fileCount: 310,
};

export const mockPresetVaults: Array<{ name: string; path: string }> = [
  { name: 'Documents', path: '~/Documents' },
  { name: 'Desktop', path: '~/Desktop' },
  { name: 'Developer', path: '~/Developer' },
  { name: 'Pictures', path: '~/Pictures' },
];
