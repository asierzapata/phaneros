import { isTauri } from '@tauri-apps/api/core';
import { invoke } from '@tauri-apps/api/core';
import { BinaryMetadataDiff, CodeDiff, DriveVault, FileNode, TelemetryMetrics } from '@/types';

export interface ConflictSummary {
  id: string;
  filename: string;
  isBinary: boolean;
  conflictKind: 'modify' | 'delete';
}

export type ConflictDiff =
  | { kind: 'text'; diff: CodeDiff }
  | { kind: 'binary'; diff: BinaryMetadataDiff };

/**
 * Thin wrapper around the Tauri `invoke()` commands that talk to a running
 * `phanerosd` over `phaneros-ipc`. Every function no-ops (returns `null`) in
 * non-Tauri contexts (tests, browser preview) so callers fall back to mocks,
 * following the same guard pattern as `trayBridge.ts`.
 */

export const fetchVaults = async (): Promise<DriveVault[] | null> => {
  if (!isTauri()) return null;
  return invoke<DriveVault[]>('list_vaults');
};

export const fetchTelemetry = async (): Promise<TelemetryMetrics | null> => {
  if (!isTauri()) return null;
  return invoke<TelemetryMetrics>('get_telemetry');
};

export const triggerSync = async (): Promise<void> => {
  if (!isTauri()) return;
  await invoke('trigger_sync');
};

export const fetchFileTree = async (vaultPath: string): Promise<FileNode[] | null> => {
  if (!isTauri()) return null;
  return invoke<FileNode[]>('get_file_tree', { path: vaultPath });
};

export const fetchConflicts = async (vaultPath: string): Promise<ConflictSummary[] | null> => {
  if (!isTauri()) return null;
  return invoke<ConflictSummary[]>('list_conflicts', { vaultPath });
};

export const fetchConflictDiff = async (conflictId: string): Promise<ConflictDiff | null> => {
  if (!isTauri()) return null;
  return invoke<ConflictDiff>('get_conflict_diff', { conflictId });
};

export const resolveConflict = async (conflictId: string, keepLocal: boolean): Promise<void> => {
  if (!isTauri()) return;
  await invoke('resolve_conflict', { conflictId, keepLocal });
};
