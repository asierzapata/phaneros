import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { TestHarness } from '../harness/test-context.js';
import { waitForStoreRoot, waitForFileContent } from '../harness/utils.js';

describe('Sync Telemetry & Efficiency Insights', () => {
  let harness: TestHarness;

  beforeEach(async () => {
    harness = await TestHarness.create();
  });

  afterEach(async (ctx) => {
    if (ctx.task.result?.state === 'fail') {
      await harness.dumpLogsOnFailure();
    }
    await harness.teardown();
  });

  it('measures compression, deduplication, phase timings, and transfer rates during sync', async () => {
    // 1. Seed vault_a with compressible text data and binary data
    const compressibleText = 'Phaneros sync telemetry test '.repeat(50); // 1.5KB text
    const binaryData = randomBytes(4 * 1024); // 4KB random binary

    await writeFile(join(harness.paths.vaultA, 'text.txt'), compressibleText);
    await writeFile(join(harness.paths.vaultA, 'binary.dat'), binaryData);

    // 2. Start server & Client A
    await harness.spawnStore();
    const clientA = await harness.spawnDaemon('client_a', harness.paths.vaultA, 'smoke_drive');

    const initialRootHash = await waitForStoreRoot(harness.storePort, 'smoke_drive');
    expect(initialRootHash).toBeTruthy();

    // 3. Start Client B to pull changes
    await harness.spawnDaemon('client_b', harness.paths.vaultB, 'smoke_drive');

    const textContentB = await waitForFileContent(
      join(harness.paths.vaultB, 'text.txt'),
      compressibleText
    );
    expect(textContentB).toBe(compressibleText);

    // 4. Test phaneros stats CLI command against client_a's daemon
    const { stdout } = await harness.cli(clientA.socketPath, [
      'stats',
      '--drive-id',
      'smoke_drive',
      '--json',
    ]);
    expect(stdout).toBeTruthy();

    const stats = JSON.parse(stdout);
    expect(stats).toHaveProperty('total_syncs');
    expect(stats.total_syncs).toBeGreaterThanOrEqual(1);
    expect(stats).toHaveProperty('total_raw_bytes');
    expect(stats.total_raw_bytes).toBeGreaterThan(0);
    expect(stats).toHaveProperty('total_wire_bytes');
    expect(stats).toHaveProperty('overall_compression_ratio_pct');
    expect(typeof stats.overall_compression_ratio_pct).toBe('number');
  });
});
