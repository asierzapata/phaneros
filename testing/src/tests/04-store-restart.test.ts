import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { TestHarness } from '../harness/test-context.js';
import {
  waitForStoreRoot,
  waitForFileContent,
  waitForServer,
} from '../harness/utils.js';

describe('Store Server Crash & Reconnect Recovery', () => {
  let harness: TestHarness;

  beforeEach(async () => {
    harness = await TestHarness.create();
  });

  afterEach(async (ctx) => {
    if (ctx.task?.result?.state === 'fail') {
      await harness.dumpLogsOnFailure();
    }
    await harness.teardown();
  });

  it('resumes synchronization seamlessly after store server restart', async () => {
    // 1. Initial setup
    await writeFile(join(harness.paths.vaultA, 'initial.txt'), 'Before crash');
    let storeManaged = await harness.spawnStore('store_initial');

    harness.spawnClient('client_a', harness.paths.vaultA, 'smoke_drive');
    await waitForStoreRoot(harness.storePort, 'smoke_drive');

    harness.spawnClient('client_b', harness.paths.vaultB, 'smoke_drive');
    await waitForFileContent(join(harness.paths.vaultB, 'initial.txt'), 'Before crash');

    // 2. Kill store process
    await harness.killManagedProcess(storeManaged);

    // 3. Restart store process on same port and DB
    storeManaged = harness.spawnManagedProcess('store_restarted', harness.storeBin, [], {
      env: {
        PORT: String(harness.storePort),
        HOST: '127.0.0.1',
        DATABASE_PATH: harness.paths.dbPath,
        BLOB_STORAGE_PATH: harness.paths.storeBlobsDir,
        PUBLIC_URL: harness.storeUrl,
      },
    });

    await waitForServer(harness.storePort);

    // 4. Perform live write after store restart
    await writeFile(join(harness.paths.vaultA, 'after_restart.txt'), 'Synced after server restart');

    const afterRestartB = await waitForFileContent(
      join(harness.paths.vaultB, 'after_restart.txt'),
      'Synced after server restart',
      25000
    );

    expect(afterRestartB).toBe('Synced after server restart');
  });
});
