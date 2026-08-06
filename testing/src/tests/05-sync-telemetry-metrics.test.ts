import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { TestHarness } from '../harness/test-context.js';
import {
  waitForStoreRoot,
  waitForFileContent,
  waitForBufferMatch,
  waitForCondition,
} from '../harness/utils.js';

async function fetchStats(
  harness: TestHarness,
  socketPath: string,
  driveId: string
): Promise<Record<string, unknown>> {
  const { stdout } = await harness.cli(socketPath, ['stats', '--drive-id', driveId, '--json']);
  return JSON.parse(stdout);
}

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

    // 4. Test phaneros stats CLI command against client_a's daemon.
    // client_a flips the remote root (making the file visible to client_b)
    // before it finishes writing its own telemetry row, so client_b
    // finishing its pull doesn't guarantee client_a's stats are updated yet
    // -- poll until they are.
    let stats: Record<string, unknown> = {};
    await waitForCondition(
      async () => {
        stats = await fetchStats(harness, clientA.socketPath, 'smoke_drive');
        return (stats.total_wire_bytes as number) > 0;
      },
      10000,
      150,
      'client_a stats to report non-zero wire bytes'
    );

    expect(stats).toHaveProperty('total_syncs');
    expect(stats.total_syncs).toBeGreaterThanOrEqual(1);
    expect(stats).toHaveProperty('total_raw_bytes');
    expect(stats.total_raw_bytes).toBeGreaterThan(0);

    // Wire bytes and compression ratio must be non-zero and wire bytes must
    // actually be smaller than the raw payload for compressible content --
    // these were the exact fields that silently regressed to 0 after the
    // ureq->reqwest async migration dropped zstd compression and the
    // record_bytes_sent/record_blob_compressed telemetry calls.
    expect(stats).toHaveProperty('total_wire_bytes');
    expect(stats.total_wire_bytes).toBeGreaterThan(0);
    expect(stats.total_wire_bytes).toBeLessThan(stats.total_raw_bytes);

    expect(stats).toHaveProperty('overall_compression_ratio_pct');
    expect(typeof stats.overall_compression_ratio_pct).toBe('number');
    expect(stats.overall_compression_ratio_pct).toBeGreaterThan(0);

    expect(stats).toHaveProperty('avg_upload_rate_bps');
    expect(stats.avg_upload_rate_bps).toBeGreaterThan(0);
  });

  it('reports deduplicated bytes saved when a new file shares content chunks with an already-synced one', async () => {
    // Whole-file-identical content collapses to node-level dedup (the File
    // node hash is a pure function of its chunk hashes, so a byte-identical
    // second file gets the exact same node hash and the diff never even
    // looks at its blobs) -- that path can't exercise blob-level dedup
    // telemetry. Blob-level dedup only shows up when a *different* file
    // shares some, but not all, content-defined chunks with something
    // already on the target -- e.g. a large file with a small localized
    // edit, mirroring FastCDC's boundary-preserving property already
    // covered at the unit level by
    // phaneros-core::scanner::tests::fastcdc_deduplication_on_insertion.
    const base = randomBytes(3 * 1024 * 1024); // 3MB, average chunk size is 1MB -> multiple chunks
    const edited = Buffer.concat([base.subarray(0, 10_000), randomBytes(100), base.subarray(10_000)]);

    await writeFile(join(harness.paths.vaultA, 'original.bin'), base);

    await harness.spawnStore();
    const clientA = await harness.spawnDaemon('client_a', harness.paths.vaultA, 'smoke_drive');
    await harness.spawnDaemon('client_b', harness.paths.vaultB, 'smoke_drive');

    await waitForBufferMatch(join(harness.paths.vaultB, 'original.bin'), base);

    // A new, distinct file (different overall hash) that nonetheless shares
    // most of its content-defined chunks with original.bin already on the
    // target -- this is the scenario that should produce dedup savings.
    await writeFile(join(harness.paths.vaultA, 'edited.bin'), edited);
    await waitForBufferMatch(join(harness.paths.vaultB, 'edited.bin'), edited, 30000);

    // Same race as above: client_a's own telemetry write trails behind the
    // root flip that lets client_b finish pulling, so poll rather than
    // reading once.
    let stats: Record<string, unknown> = {};
    await waitForCondition(
      async () => {
        stats = await fetchStats(harness, clientA.socketPath, 'smoke_drive');
        return (stats.total_dedup_bytes as number) > 0;
      },
      10000,
      150,
      'client_a stats to report non-zero dedup bytes'
    );

    expect(stats).toHaveProperty('total_dedup_bytes');
    expect(stats.total_dedup_bytes).toBeGreaterThan(0);
  });
});
