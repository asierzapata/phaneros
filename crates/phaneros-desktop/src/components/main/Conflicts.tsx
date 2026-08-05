import React, { useEffect, useState } from 'react';
import { useVault } from '@/context/VaultContext';
import { DiffView } from '@/components/ui/DiffView';
import { mockTextConflict, mockBinaryConflict } from '@/__tests__/mocks/diffMocks';
import {
  ConflictDiff,
  ConflictSummary,
  fetchConflictDiff,
  fetchConflicts,
  resolveConflict,
} from '@/lib/backendBridge';

export const Conflicts: React.FC = () => {
  const { activeDrive } = useVault();

  // Mock-driven fallback UI (non-Tauri / no real conflicts detected yet).
  const [mockConflictType, setMockConflictType] = useState<'text' | 'binary'>('text');
  const [resolvedStatus, setResolvedStatus] = useState<string | null>(null);

  // Real, filesystem-backed conflicts (populated only inside Tauri).
  const [conflicts, setConflicts] = useState<ConflictSummary[]>([]);
  const [selectedConflictId, setSelectedConflictId] = useState<string | null>(null);
  const [selectedDiff, setSelectedDiff] = useState<ConflictDiff | null>(null);

  useEffect(() => {
    if (!activeDrive) return;
    let cancelled = false;
    fetchConflicts(activeDrive.path)
      .then((list) => {
        if (cancelled || list === null) return;
        setConflicts(list);
        setSelectedConflictId((current) => current ?? list[0]?.id ?? null);
      })
      .catch(() => {
        // Non-fatal: fall back to the mock-driven UI below.
      });
    return () => {
      cancelled = true;
    };
  }, [activeDrive]);

  useEffect(() => {
    if (!selectedConflictId) {
      setSelectedDiff(null);
      return;
    }
    let cancelled = false;
    fetchConflictDiff(selectedConflictId)
      .then((diff) => {
        if (!cancelled) setSelectedDiff(diff);
      })
      .catch(() => {
        if (!cancelled) setSelectedDiff(null);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedConflictId]);

  const usingRealConflicts = conflicts.length > 0;

  const resolveSelected = async (keepLocal: boolean) => {
    if (!selectedConflictId) return;
    try {
      await resolveConflict(selectedConflictId, keepLocal);
      const remaining = conflicts.filter((c) => c.id !== selectedConflictId);
      setConflicts(remaining);
      setSelectedConflictId(remaining[0]?.id ?? null);
      setResolvedStatus(keepLocal ? 'Resolved: Kept Local Copy' : 'Resolved: Kept Server Copy');
    } catch {
      // Leave the conflict in place; the user can retry.
    }
  };

  const handleKeepLocal = () => {
    if (usingRealConflicts) {
      void resolveSelected(true);
    } else {
      setResolvedStatus('Resolved: Kept Local Copy');
    }
  };

  const handleKeepServer = () => {
    if (usingRealConflicts) {
      void resolveSelected(false);
    } else {
      setResolvedStatus('Resolved: Kept Server Copy');
    }
  };

  return (
    <div className="flex flex-col gap-4 p-6 max-w-6xl mx-auto font-sans" data-testid="conflicts-workspace">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-serif font-bold text-foreground">Conflict Resolution Workspace</h2>
          <p className="text-sm text-muted-foreground">Review and resolve differences between local and server files.</p>
        </div>

        {usingRealConflicts ? (
          <div className="flex items-center gap-2" data-testid="conflict-selector-list">
            {conflicts.map((conflict) => (
              <button
                key={conflict.id}
                type="button"
                onClick={() => setSelectedConflictId(conflict.id)}
                className={`px-3 py-1.5 text-xs font-semibold rounded ${
                  conflict.id === selectedConflictId
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted text-muted-foreground'
                }`}
                data-testid={`conflict-item-${conflict.id}`}
              >
                {conflict.filename}
              </button>
            ))}
          </div>
        ) : (
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setMockConflictType('text')}
              className={`px-3 py-1.5 text-xs font-semibold rounded ${
                mockConflictType === 'text' ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground'
              }`}
              data-testid="switch-text-conflict"
            >
              Text Diff (README.md)
            </button>
            <button
              type="button"
              onClick={() => setMockConflictType('binary')}
              className={`px-3 py-1.5 text-xs font-semibold rounded ${
                mockConflictType === 'binary' ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground'
              }`}
              data-testid="switch-binary-conflict"
            >
              Binary Matrix (database.sqlite)
            </button>
          </div>
        )}
      </div>

      {resolvedStatus && (
        <div className="p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-lg text-emerald-700 dark:text-emerald-300 text-xs font-semibold" data-testid="resolution-banner">
          {resolvedStatus}
        </div>
      )}

      {usingRealConflicts ? (
        selectedDiff && (
          <DiffView
            textDiff={selectedDiff.kind === 'text' ? selectedDiff.diff : undefined}
            binaryDiff={selectedDiff.kind === 'binary' ? selectedDiff.diff : undefined}
            onKeepLocal={handleKeepLocal}
            onKeepServer={handleKeepServer}
          />
        )
      ) : mockConflictType === 'text' ? (
        <DiffView textDiff={mockTextConflict} onKeepLocal={handleKeepLocal} onKeepServer={handleKeepServer} />
      ) : (
        <DiffView binaryDiff={mockBinaryConflict} onKeepLocal={handleKeepLocal} onKeepServer={handleKeepServer} />
      )}
    </div>
  );
};
