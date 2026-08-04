import { createServer } from 'node:net';
import { readFile, readdir } from 'node:fs/promises';
import type { ManagedProcess } from './types.js';

export async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function getFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      if (typeof address === 'object' && address !== null) {
        const port = address.port;
        server.close(() => resolve(port));
      } else {
        reject(new Error('Failed to obtain a free port'));
      }
    });
  });
}

export async function waitForCondition(
  conditionFn: () => Promise<boolean>,
  timeoutMs = 15000,
  intervalMs = 150,
  label = 'condition'
): Promise<void> {
  const startTime = Date.now();
  while (Date.now() - startTime < timeoutMs) {
    try {
      if (await conditionFn()) return;
    } catch {
      // Ignored during polling
    }
    await sleep(intervalMs);
  }
  throw new Error(`Timeout waiting for ${label} after ${timeoutMs}ms`);
}

export async function waitForFileContent(
  filePath: string,
  expectedContent: string,
  timeoutMs = 15000
): Promise<string> {
  let actualContent = '';
  await waitForCondition(
    async () => {
      try {
        actualContent = await readFile(filePath, 'utf-8');
        return actualContent === expectedContent;
      } catch {
        return false;
      }
    },
    timeoutMs,
    150,
    `file content at ${filePath} to match "${expectedContent}"`
  );
  return actualContent;
}

export async function waitForFileAbsence(filePath: string, timeoutMs = 15000): Promise<void> {
  await waitForCondition(
    async () => {
      try {
        await readFile(filePath);
        return false;
      } catch {
        return true;
      }
    },
    timeoutMs,
    150,
    `absence of file at ${filePath}`
  );
}

export async function waitForBufferMatch(
  filePath: string,
  expectedBuffer: Buffer,
  timeoutMs = 15000
): Promise<Buffer> {
  let actualBuffer = Buffer.alloc(0);
  await waitForCondition(
    async () => {
      try {
        actualBuffer = await readFile(filePath);
        return expectedBuffer.equals(actualBuffer);
      } catch {
        return false;
      }
    },
    timeoutMs,
    150,
    `binary file match at ${filePath}`
  );
  return actualBuffer;
}

export async function waitForDirectoryFileCount(
  dirPath: string,
  expectedCount: number,
  timeoutMs = 15000
): Promise<number> {
  let fileCount = 0;
  await waitForCondition(
    async () => {
      try {
        const entries = await readdir(dirPath, { recursive: true, withFileTypes: true });
        fileCount = entries.filter((entry) => entry.isFile()).length;
        return fileCount >= expectedCount;
      } catch {
        return false;
      }
    },
    timeoutMs,
    150,
    `directory ${dirPath} to contain at least ${expectedCount} files (current: ${fileCount})`
  );
  return fileCount;
}

export async function waitForServer(port: number, timeoutMs = 10000): Promise<void> {
  await waitForCondition(
    async () => {
      const res = await fetch(`http://127.0.0.1:${port}/health`);
      return res.ok;
    },
    timeoutMs,
    150,
    `server at port ${port} to be healthy`
  );
}

export async function waitForStoreRoot(port: number, driveId: string, timeoutMs = 15000): Promise<string> {
  let rootHash = '';
  await waitForCondition(
    async () => {
      const res = await fetch(`http://127.0.0.1:${port}/api/drives/${driveId}/root`);
      if (res.ok) {
        const data = (await res.json()) as { hash: string };
        rootHash = data.hash;
        return Boolean(rootHash);
      }
      return false;
    },
    timeoutMs,
    150,
    `root hash for drive ${driveId} to be set on remote store`
  );
  return rootHash;
}

export async function dumpLogs(managedProcs: ManagedProcess[]): Promise<void> {
  console.error('\n📋 Process Log Tails (Failure Diagnostics):');
  for (const item of managedProcs) {
    try {
      const content = await readFile(item.logFile, 'utf-8');
      const lines = content.trim().split('\n').slice(-20).join('\n');
      console.error(`\n--- Log tail for ${item.name} (${item.logFile}) ---`);
      console.error(lines || '(empty log)');
    } catch {
      // Log file may not exist
    }
  }
}

