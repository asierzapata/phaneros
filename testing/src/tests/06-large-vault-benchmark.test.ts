import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { TestHarness } from '../harness/test-context.js';
import {
  waitForStoreRoot,
  waitForFileContent,
  waitForDirectoryFileCount,
} from '../harness/utils.js';

describe('Large Vault Benchmark', { timeout: 120_000 }, () => {
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

  it('measures wide tree push and pull performance (200 small files)', { timeout: 60_000 }, async () => {
    // 1. Create 200 files in vault A (~100 bytes each)
    const fileCount = 200;
    for (let i = 0; i < fileCount; i++) {
      const fileName = `wide_file_${String(i).padStart(3, '0')}.txt`;
      const content = `Benchmark wide tree file #${i} content padding line to ensure size is around 100 bytes total payload.`;
      await writeFile(join(harness.paths.vaultA, fileName), content);
    }

    // 2. Start store & Client A
    await harness.spawnStore();
    const pushStart = performance.now();
    await harness.spawnDaemon('client_a', harness.paths.vaultA, 'smoke_drive');

    const rootHash = await waitForStoreRoot(harness.storePort, 'smoke_drive', 60_000);
    const pushTime = performance.now() - pushStart;
    expect(rootHash).toBeTruthy();

    // 3. Start Client B and measure pull time
    const pullStart = performance.now();
    await harness.spawnDaemon('client_b', harness.paths.vaultB, 'smoke_drive');

    const syncedCount = await waitForDirectoryFileCount(harness.paths.vaultB, fileCount, 60_000);
    const pullTime = performance.now() - pullStart;

    expect(syncedCount).toBeGreaterThanOrEqual(fileCount);

    const sampleContent = await readFile(join(harness.paths.vaultB, 'wide_file_000.txt'), 'utf-8');
    expect(sampleContent).toContain('Benchmark wide tree file #0');

    console.log(`[Wide Tree Benchmark] 200 files - Push time: ${pushTime.toFixed(2)}ms, Pull time: ${pullTime.toFixed(2)}ms`);
  });

  it('measures deep tree push and pull performance (5 levels of nesting)', { timeout: 60_000 }, async () => {
    let totalFiles = 0;

    async function populateDeepTree(currentDir: string, currentLevel: number, maxLevel: number): Promise<void> {
      if (currentLevel > maxLevel) return;
      for (let f = 0; f < 3; f++) {
        const subDir = join(currentDir, `level_${currentLevel}_dir_${f}`);
        await mkdir(subDir, { recursive: true });
        for (let i = 0; i < 2; i++) {
          const content = `Deep tree level ${currentLevel} dir ${f} file ${i} payload ~100 bytes for benchmark testing.`;
          await writeFile(join(subDir, `file_${i}.txt`), content);
          totalFiles++;
        }
        await populateDeepTree(subDir, currentLevel + 1, maxLevel);
      }
    }

    await populateDeepTree(harness.paths.vaultA, 1, 5);

    await harness.spawnStore();
    const pushStart = performance.now();
    await harness.spawnDaemon('client_a', harness.paths.vaultA, 'smoke_drive');

    const rootHash = await waitForStoreRoot(harness.storePort, 'smoke_drive', 60_000);
    const pushTime = performance.now() - pushStart;
    expect(rootHash).toBeTruthy();

    const pullStart = performance.now();
    await harness.spawnDaemon('client_b', harness.paths.vaultB, 'smoke_drive');

    const syncedCount = await waitForDirectoryFileCount(harness.paths.vaultB, totalFiles, 60_000);
    const pullTime = performance.now() - pullStart;

    expect(syncedCount).toBeGreaterThanOrEqual(totalFiles);

    console.log(`[Deep Tree Benchmark] ${totalFiles} nested files - Push time: ${pushTime.toFixed(2)}ms, Pull time: ${pullTime.toFixed(2)}ms`);
  });

  it('measures incremental sync timing after modifying 1 file in a 200-file vault', { timeout: 60_000 }, async () => {
    const fileCount = 200;
    for (let i = 0; i < fileCount; i++) {
      const fileName = `inc_file_${String(i).padStart(3, '0')}.txt`;
      const content = `Incremental benchmark file #${i} initial content padding around 100 bytes long text.`;
      await writeFile(join(harness.paths.vaultA, fileName), content);
    }

    await harness.spawnStore();
    await harness.spawnDaemon('client_a', harness.paths.vaultA, 'smoke_drive');
    await waitForStoreRoot(harness.storePort, 'smoke_drive', 60_000);

    await harness.spawnDaemon('client_b', harness.paths.vaultB, 'smoke_drive');
    await waitForDirectoryFileCount(harness.paths.vaultB, fileCount, 60_000);

    // Now change just 1 file
    const targetFile = 'inc_file_100.txt';
    const modifiedContent = `MODIFIED content for file #100 at timestamp ${Date.now()}`;

    const incStart = performance.now();
    await writeFile(join(harness.paths.vaultA, targetFile), modifiedContent);

    const receivedContent = await waitForFileContent(
      join(harness.paths.vaultB, targetFile),
      modifiedContent,
      60_000
    );
    const incTime = performance.now() - incStart;

    expect(receivedContent).toBe(modifiedContent);

    console.log(`[Incremental Sync Benchmark] Re-syncing 1 modified file among ${fileCount} files took: ${incTime.toFixed(2)}ms`);
  });

  it('measures bootstrap pull timing for a complete 200-file vault from scratch', { timeout: 60_000 }, async () => {
    const fileCount = 200;
    for (let i = 0; i < fileCount; i++) {
      const fileName = `boot_file_${String(i).padStart(3, '0')}.txt`;
      const content = `Bootstrap pull benchmark file #${i} padding content around 100 bytes length payload.`;
      await writeFile(join(harness.paths.vaultA, fileName), content);
    }

    await harness.spawnStore();
    await harness.spawnDaemon('client_a', harness.paths.vaultA, 'smoke_drive');
    await waitForStoreRoot(harness.storePort, 'smoke_drive', 60_000);

    const bootstrapStart = performance.now();
    await harness.spawnDaemon('client_b', harness.paths.vaultB, 'smoke_drive');

    const syncedCount = await waitForDirectoryFileCount(harness.paths.vaultB, fileCount, 60_000);
    const bootstrapTime = performance.now() - bootstrapStart;

    expect(syncedCount).toBeGreaterThanOrEqual(fileCount);

    const sampleContent = await readFile(join(harness.paths.vaultB, 'boot_file_199.txt'), 'utf-8');
    expect(sampleContent).toContain('Bootstrap pull benchmark file #199');

    console.log(`[Bootstrap Pull Benchmark] Pulling complete ${fileCount}-file vault from scratch took: ${bootstrapTime.toFixed(2)}ms`);
  });
});
