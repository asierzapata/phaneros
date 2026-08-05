import { CodeDiff, BinaryMetadataDiff } from '@/types';

export const mockTextConflict: CodeDiff = {
  filename: 'README.md',
  path: '~/Documents/PhanerosSync/README.md',
  linesAdded: 3,
  linesRemoved: 2,
  chunks: [
    {
      oldStart: 10,
      newStart: 10,
      lines: [
        { type: 'same', text: '## Synchronization Status' },
        {
          type: 'delete',
          text: '- Local encryption active',
          wordHighlights: [{ type: 'delete', word: 'Local' }],
        },
        {
          type: 'add',
          text: '+ End-to-end local encryption active',
          wordHighlights: [{ type: 'add', word: 'End-to-end' }],
        },
      ],
    },
  ],
};

export const mockCodeDiffRs: CodeDiff = {
  filename: 'vault.rs',
  path: '~/Developer/Projects/src/vault.rs',
  linesAdded: 5,
  linesRemoved: 1,
  chunks: [
    {
      oldStart: 1,
      newStart: 1,
      lines: [
        { type: 'same', text: 'pub struct Vault {' },
        { type: 'delete', text: '    pub quota: u64,', wordHighlights: [{ type: 'delete', word: 'u64' }] },
        { type: 'add', text: '    pub quota: Option<u64>, // Infinite quota when None', wordHighlights: [{ type: 'add', word: 'Option<u64>' }] },
        { type: 'same', text: '}' },
      ],
    },
  ],
};

export const mockBinaryConflict: BinaryMetadataDiff = {
  filename: 'database.sqlite',
  path: '~/Documents/PhanerosSync/database.sqlite',
  isBinary: true,
  local: {
    size: '14.2 MB',
    modified: '2026-08-04 22:10',
    hash: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
  },
  store: {
    size: '15.8 MB',
    modified: '2026-08-04 22:12',
    hash: 'f4c1d553090d2d250b0cf5d9007gc03538bf52f5750c045db506002c8963c966',
  },
  recommendedAction: 'Keep Local',
};

export const mockBinaryConflictStoreRecommended: BinaryMetadataDiff = {
  filename: 'archive.zip',
  path: '~/Documents/PhanerosSync/archive.zip',
  isBinary: true,
  local: {
    size: '102.4 MB',
    modified: '2026-08-04 20:00',
    hash: 'a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890',
  },
  store: {
    size: '105.1 MB',
    modified: '2026-08-04 22:30',
    hash: 'b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1',
  },
  recommendedAction: 'Keep Store',
};
