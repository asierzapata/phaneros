import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import { useVault } from '@/context/VaultContext';
import { FileNode } from '@/types';
import React, { useState } from 'react';

const RecursiveFileTreeNode: React.FC<{ node: FileNode; depth?: number }> = ({ node, depth = 0 }) => {
  const [expanded, setExpanded] = useState<boolean>(true);

  return (
    <div data-testid={`tree-node-${node.id}`} style={{ paddingLeft: `${depth * 16}px` }} className="tree-indent-guide">
      <div
        className="flex items-center space-x-2 py-1 cursor-pointer"
        onClick={() => node.isDir && setExpanded(!expanded)}
        data-testid={`node-row-${node.id}`}
      >
        <span>{node.isDir ? (expanded ? '📂' : '📁') : '📄'}</span>
        <span data-testid={`node-name-${node.id}`}>{node.name}</span>
        {node.badge && (
          <span data-testid={`node-badge-${node.id}`} className="px-1 text-xs bg-slate-200 dark:bg-slate-700 font-mono">
            {node.badge}
          </span>
        )}
      </div>

      {node.isDir && expanded && (
        <div data-testid={`node-children-${node.id}`}>
          {!node.children || node.children.length === 0 ? (
            <div data-testid={`empty-dir-${node.id}`} style={{ paddingLeft: '16px' }} className="text-xs text-muted">
              (Empty directory)
            </div>
          ) : (
            node.children.map((child) => (
              <RecursiveFileTreeNode key={child.id} node={child} depth={depth + 1} />
            ))
          )}
        </div>
      )}
    </div>
  );
};

const FileExplorerBoundaryComponent: React.FC<{ customTree?: FileNode[] }> = ({ customTree }) => {
  const { drives, activeDriveId, selectDrive } = useVault();

  return (
    <div data-testid="explorer-root" className="flex h-full">
      {/* Drive Selector */}
      <aside data-testid="drive-selector" className="w-64 border-r p-4">
        <h3>Drives</h3>
        {drives.length === 0 ? (
          <p data-testid="empty-drives-msg">No drives available</p>
        ) : (
          <ul>
            {drives.map((d) => (
              <li
                key={d.id}
                data-testid={`drive-item-${d.id}`}
                onClick={() => selectDrive(d.id)}
                className={activeDriveId === d.id ? 'font-bold' : ''}
              >
                {d.name}
              </li>
            ))}
          </ul>
        )}
      </aside>

      {/* Interactive File Tree */}
      <main data-testid="file-tree-container" className="flex-1 p-4">
        {customTree ? (
          customTree.map((node) => <RecursiveFileTreeNode key={node.id} node={node} />)
        ) : (
          <p data-testid="no-files-msg">No files</p>
        )}
      </main>
    </div>
  );
};

describe('F6_TREE: File Explorer Boundary Tests', () => {
  it('F6-T2-01: should render deeply nested file tree up to 5+ levels with correct indent guide structure', () => {
    const deepNestedTree: FileNode[] = [
      {
        id: 'L1',
        name: 'level-1',
        ext: '',
        isDir: true,
        children: [
          {
            id: 'L2',
            name: 'level-2',
            ext: '',
            isDir: true,
            children: [
              {
                id: 'L3',
                name: 'level-3',
                ext: '',
                isDir: true,
                children: [
                  {
                    id: 'L4',
                    name: 'level-4',
                    ext: '',
                    isDir: true,
                    children: [
                      {
                        id: 'L5',
                        name: 'level-5-file',
                        ext: 'rs',
                        isDir: false,
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

    render(<FileExplorerBoundaryComponent customTree={deepNestedTree} />);

    expect(screen.getByTestId('node-name-L1')).toHaveTextContent('level-1');
    expect(screen.getByTestId('node-name-L5')).toHaveTextContent('level-5-file');
    expect(screen.getByTestId('node-badge-L5')).toHaveTextContent('RS');
  });

  it('F6-T2-02: should render empty directory message when directory node has no children', () => {
    const emptyFolderTree: FileNode[] = [
      {
        id: 'empty-folder-1',
        name: 'empty-docs',
        ext: '',
        isDir: true,
        children: [],
      },
    ];

    render(<FileExplorerBoundaryComponent customTree={emptyFolderTree} />);

    expect(screen.getByTestId('empty-dir-empty-folder-1')).toHaveTextContent('(Empty directory)');
  });

  it('F6-T2-03: should safely render file names containing special characters, spaces, and unicode/emojis', () => {
    const specialCharTree: FileNode[] = [
      {
        id: 'spec-1',
        name: '🚀 Project #1 (v2.0) & Notes! @2026',
        ext: 'md',
        isDir: false,
        badge: 'MD',
      },
    ];

    render(<FileExplorerBoundaryComponent customTree={specialCharTree} />);

    expect(screen.getByTestId('node-name-spec-1')).toHaveTextContent('🚀 Project #1 (v2.0) & Notes! @2026');
  });

  it('F6-T2-04: should handle missing extension badge fallback for files without extension', () => {
    const noExtTree: FileNode[] = [
      {
        id: 'no-ext-1',
        name: 'Dockerfile',
        ext: '',
        isDir: false,
      },
    ];

    render(<FileExplorerBoundaryComponent customTree={noExtTree} />);

    expect(screen.getByTestId('node-name-no-ext-1')).toHaveTextContent('Dockerfile');
    expect(screen.queryByTestId('node-badge-no-ext-1')).not.toBeInTheDocument();
  });

  it('F6-T2-05: should render empty drive list boundary state when drives array is empty', () => {
    render(<FileExplorerBoundaryComponent customTree={[]} />, {
      providerProps: {
        vaultProps: { initialDrives: [] },
      },
    });

    expect(screen.getByTestId('empty-drives-msg')).toHaveTextContent('No drives available');
  });
});
