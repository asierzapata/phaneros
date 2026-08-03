import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { TestHarness } from '../harness/test-context.js';
import { waitForStoreRoot, sleep } from '../harness/utils.js';

describe('Multi-Tenant Drive Isolation', () => {
  let harness: TestHarness;

  beforeEach(async () => {
    harness = await TestHarness.create();
  });

  afterEach(async () => {
    await harness.teardown();
  });

  it('prevents data leakage between separate drives', async () => {
    // 1. Setup store, Client A for smoke_drive, and write files in vaultA
    await writeFile(join(harness.paths.vaultA, 'public.txt'), 'Public Drive Data');
    await harness.spawnStore();

    harness.spawnClient('client_a', harness.paths.vaultA, 'smoke_drive');
    await waitForStoreRoot(harness.storePort, 'smoke_drive');

    // 2. Setup Client C for isolated_drive with secret file in vaultC
    await writeFile(join(harness.paths.vaultC, 'secret.txt'), 'Top Secret Isolated Data');
    harness.spawnClient('client_c', harness.paths.vaultC, 'isolated_drive');

    const isolatedRootHash = await waitForStoreRoot(harness.storePort, 'isolated_drive');
    expect(isolatedRootHash).toBeTruthy();

    // 3. Confirm secret.txt is NOT leaked into vault_a
    await sleep(1000);
    let secretLeakedInA = false;
    try {
      await readFile(join(harness.paths.vaultA, 'secret.txt'));
      secretLeakedInA = true;
    } catch {
      secretLeakedInA = false;
    }

    expect(secretLeakedInA).toBe(false);
  });
});
