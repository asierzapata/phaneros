export interface ThemeState {
  theme: 'light' | 'dark';
  toggleTheme: () => void;
  setTheme: (theme: 'light' | 'dark') => void;
}

export interface DriveVault {
  id: string;
  name: string;
  path: string;
  status: 'synced' | 'syncing' | 'conflict' | 'paused';
  usedBytes?: number;
  quotaBytes?: number; // Display as Infinite (∞ / Unlimited) when undefined or null
  fileCount?: number;
}

export interface TelemetryMetrics {
  lastSynced: string;
  deduplicationRatio: string;
  compressionRatio: string;
  transferSpeed: string;
}

export interface FileNode {
  id: string;
  name: string;
  ext: string;
  isDir: boolean;
  size?: string;
  modified?: string;
  children?: FileNode[];
  badge?: string;
}

export interface CodeDiffLine {
  type: 'same' | 'add' | 'delete';
  text: string;
  wordHighlights?: Array<{ type: 'add' | 'delete'; word: string }>;
}

export interface CodeDiffChunk {
  oldStart: number;
  newStart: number;
  lines: CodeDiffLine[];
}

export interface CodeDiff {
  filename: string;
  path: string;
  linesAdded: number;
  linesRemoved: number;
  chunks: CodeDiffChunk[];
}

export interface BinaryMetadataDiff {
  filename: string;
  path: string;
  isBinary: true;
  local: { size: string; modified: string; hash: string };
  store: { size: string; modified: string; hash: string };
  recommendedAction: 'Keep Local' | 'Keep Store';
}

export interface OnboardingState {
  currentStep: number;
  isCompleted: boolean;
  destinationMode: 'cloud' | 'self-hosted';
  serverUrl: string;
  serverToken: string;
  isConnected: boolean;
  vaults: DriveVault[];
}

export type MainTab = 'dashboard' | 'drives' | 'conflicts' | 'activity' | 'settings';

export interface ViewState {
  activeTab: MainTab;
  setActiveTab: (tab: MainTab) => void;
}

export interface TrayActivityItem {
  id: string;
  name: string; // Clean basename (no path, no extension)
  ext: string;  // Extension pill text (e.g. 'RS', 'MD', 'DB')
  timestamp: string;
  action: 'synced' | 'modified' | 'deleted' | 'conflict';
}
