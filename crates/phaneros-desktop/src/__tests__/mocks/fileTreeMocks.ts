import { FileNode } from '@/types';

export const mockFileTree: FileNode[] = [
  {
    id: 'dir-1',
    name: 'src',
    ext: '',
    isDir: true,
    children: [
      {
        id: 'file-1',
        name: 'sync-protocol',
        ext: 'rs',
        isDir: false,
        size: '14.2 KB',
        modified: '2m ago',
        badge: 'RS',
      },
      {
        id: 'file-2',
        name: 'phaneros',
        ext: 'md',
        isDir: false,
        size: '4.8 KB',
        modified: '10m ago',
        badge: 'MD',
      },
    ],
  },
  {
    id: 'file-3',
    name: 'state-db',
    ext: 'sqlite',
    isDir: false,
    size: '18.4 MB',
    modified: '1h ago',
    badge: 'DB',
  },
];

export const mockNestedFileTree: FileNode[] = [
  {
    id: 'root-dir',
    name: 'phaneros-core',
    ext: '',
    isDir: true,
    children: [
      {
        id: 'sub-dir-1',
        name: 'crates',
        ext: '',
        isDir: true,
        children: [
          {
            id: 'sub-dir-2',
            name: 'phaneros-desktop',
            ext: '',
            isDir: true,
            children: [
              {
                id: 'sub-dir-3',
                name: 'src',
                ext: '',
                isDir: true,
                children: [
                  {
                    id: 'nested-file-1',
                    name: 'main',
                    ext: 'rs',
                    isDir: false,
                    size: '2.1 KB',
                    modified: 'Just now',
                    badge: 'RS',
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
];

export const mockEmptyFileTree: FileNode[] = [];
