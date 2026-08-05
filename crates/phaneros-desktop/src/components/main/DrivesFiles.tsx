import React, { useEffect, useState } from 'react';
import { useVault } from '@/context/VaultContext';
import { FileTree } from '@/components/ui/FileTree';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { FileNode } from '@/types';
import { mockFileTree } from '@/__tests__/mocks/fileTreeMocks';
import { fetchFileTree } from '@/lib/backendBridge';

export const DrivesFiles: React.FC = () => {
  const { drives, activeDriveId, selectDrive, activeDrive } = useVault();
  const [fileTree, setFileTree] = useState<FileNode[]>(mockFileTree);
  const [treeError, setTreeError] = useState<string | null>(null);

  useEffect(() => {
    if (!activeDrive) return;
    let cancelled = false;
    fetchFileTree(activeDrive.path)
      .then((fetched) => {
        if (cancelled || fetched === null) return;
        setFileTree(fetched);
        setTreeError(null);
      })
      .catch((err) => {
        if (cancelled) return;
        setTreeError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [activeDrive]);

  const quotaDisplay = activeDrive?.quotaBytes === undefined ? 'Infinite ∞' : `${activeDrive?.quotaBytes} B`;

  return (
    <div className="flex h-full p-6 gap-6 max-w-6xl mx-auto font-sans" data-testid="drives-files-workspace">
      {/* Left Split Drive Selector List */}
      <div className="w-1/3 flex flex-col gap-3" data-testid="drive-selector-list">
        <h3 className="text-sm font-serif font-bold text-foreground">Storage Drives</h3>
        <div className="flex flex-col gap-2">
          {drives.map((drive) => {
            const isSelected = drive.id === activeDriveId;
            return (
              <div
                key={drive.id}
                onClick={() => selectDrive(drive.id)}
                className={`p-3 rounded-lg border cursor-pointer transition-all ${
                  isSelected ? 'border-primary bg-accent/40 shadow-sm' : 'border-border bg-card hover:bg-muted'
                }`}
                data-testid={`drive-selector-item-${drive.id}`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-sm">{drive.name}</span>
                  <Badge variant={drive.status}>{drive.status}</Badge>
                </div>
                <div className="text-xs font-mono text-muted-foreground mt-1 truncate">{drive.path}</div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Right Drive Metadata Card + Interactive File Tree */}
      <div className="flex-1 flex flex-col gap-4" data-testid="drive-files-right-pane">
        {activeDrive && (
          <Card elevation="low" className="p-4 flex flex-col gap-2" data-testid="drive-metadata-card">
            <div className="flex justify-between items-center">
              <h3 className="font-serif font-bold text-base">{activeDrive.name}</h3>
              <Badge variant={activeDrive.status}>{activeDrive.status}</Badge>
            </div>
            <div className="text-xs font-mono text-muted-foreground">{activeDrive.path}</div>
            <div className="flex justify-between items-center text-xs mt-1 pt-2 border-t border-border">
              <span className="text-muted-foreground">Capacity / Quota:</span>
              <span className="font-mono font-bold text-foreground" data-testid="active-drive-quota-display">
                {quotaDisplay}
              </span>
            </div>
          </Card>
        )}

        <Card elevation="low" className="flex-1 p-4 bg-card overflow-auto">
          <h4 className="text-xs font-serif font-semibold mb-3 text-muted-foreground uppercase tracking-wider">
            File Tree Hierarchy
          </h4>
          {treeError && (
            <p className="text-xs text-red-600 dark:text-red-400 mb-2" data-testid="file-tree-error">
              {treeError}
            </p>
          )}
          <FileTree nodes={fileTree} />
        </Card>
      </div>
    </div>
  );
};
