import type { ChildProcess } from 'node:child_process';
import type { WriteStream } from 'node:fs';

export interface ManagedProcess {
  name: string;
  proc: ChildProcess;
  logFile: string;
  logStream: WriteStream;
}

export interface TestHarnessPaths {
  sandboxDir: string;
  logsDir: string;
  storeDataDir: string;
  storeBlobsDir: string;
  dbPath: string;
  configPath: string;
  vaultA: string;
  vaultB: string;
  vaultC: string;
}

export interface HarnessOptions {
  storePort?: number;
  drives?: Array<{ id: string; vaultPath: string; token?: string }>;
}
