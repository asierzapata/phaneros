import { isTauri } from '@tauri-apps/api/core';
import { invoke } from '@tauri-apps/api/core';
import { ActivitySession, BinaryMetadataDiff, CodeDiff, DriveVault, FileNode, TelemetryMetrics } from '@/types';

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

export const fetchRecentActivity = async (limit = 20): Promise<ActivitySession[] | null> => {
  if (!isTauri()) return null;
  return invoke<ActivitySession[]>('list_activity', { limit });
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

export interface OnboardingStateDto {
  isCompleted: boolean;
  destinationMode: 'cloud' | 'self-hosted';
  serverUrl: string;
}

export interface DaemonPingResult {
  version: string;
  configured: boolean;
}

/**
 * Confirms a `phanerosd` instance is reachable and reports whether it has
 * at least one drive configured. Throws if the daemon is unreachable.
 */
export const pingDaemon = async (): Promise<DaemonPingResult | null> => {
  if (!isTauri()) return null;
  return invoke<DaemonPingResult>('daemon_ping');
};

export const addVaultRemote = async (
  driveId: string,
  path: string,
  storeUrl?: string,
  token?: string
): Promise<void> => {
  if (!isTauri()) return;
  await invoke('add_vault', { driveId, path, storeUrl, token });
};

export const loadOnboardingState = async (): Promise<OnboardingStateDto | null> => {
  if (!isTauri()) return null;
  return invoke<OnboardingStateDto | null>('load_onboarding_state');
};

export const saveOnboardingState = async (state: OnboardingStateDto): Promise<void> => {
  if (!isTauri()) return;
  await invoke('save_onboarding_state', { state });
};

/** Opens a native folder picker; returns null if the user cancels or outside Tauri. */
export const pickFolder = async (): Promise<string | null> => {
  if (!isTauri()) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const result = await open({ directory: true, multiple: false });
  return typeof result === 'string' ? result : null;
};

/**
 * Spawns `phanerosd` as a detached one-off process. This is a best-effort
 * convenience for first-run/recovery — it doesn't wait for the daemon's
 * socket to come up; callers should re-poll (e.g. via `pingDaemon`) after
 * a short delay to confirm it actually started.
 */
export const startDaemon = async (): Promise<void> => {
  if (!isTauri()) return;
  await invoke('start_daemon');
};

/** Registers `phanerosd` as a per-user login item (macOS only for now). */
export const registerLoginItem = async (): Promise<void> => {
  if (!isTauri()) return;
  await invoke('register_login_item');
};

export const unregisterLoginItem = async (): Promise<void> => {
  if (!isTauri()) return;
  await invoke('unregister_login_item');
};

export const isLoginItemRegistered = async (): Promise<boolean | null> => {
  if (!isTauri()) return null;
  return invoke<boolean>('is_login_item_registered');
};
