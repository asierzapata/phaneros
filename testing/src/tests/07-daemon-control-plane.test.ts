import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, rm, writeFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { TestHarness } from '../harness/test-context.js';
import { waitForDaemonReady, waitForFileContent, waitForStoreRoot } from '../harness/utils.js';

describe('Daemon Control Plane (multi-drive, single daemon)', () => {
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

  it('lists, stops, starts, adds, and removes drives on one daemon', async () => {
    // 1. One daemon, hosting both drives declared in the shared config.
    await harness.spawnStore();
    await writeFile(join(harness.paths.vaultA, 'hello.txt'), 'Hello from smoke_drive');
    await writeFile(join(harness.paths.vaultC, 'hello.txt'), 'Hello from isolated_drive');

    harness.spawnManagedProcess('main_daemon', harness.daemonBin, [
      '--config',
      harness.paths.configPath,
    ]);
    await waitForDaemonReady(harness.cliBin, harness.mainSocketPath);

    // 2. Both drives show up and are running.
    const smokeStatus = await harness.cli(harness.mainSocketPath, [
      'status',
      '--drive-id',
      'smoke_drive',
    ]);
    expect(smokeStatus.stdout).toContain('smoke_drive');

    const isolatedStatus = await harness.cli(harness.mainSocketPath, [
      'status',
      '--drive-id',
      'isolated_drive',
    ]);
    expect(isolatedStatus.stdout).toContain('isolated_drive');

    // 3. Stop isolated_drive; smoke_drive keeps syncing live edits.
    await harness.cli(harness.mainSocketPath, ['stop', 'isolated_drive']);
    const stoppedStatus = await harness.cli(harness.mainSocketPath, [
      'status',
      '--drive-id',
      'isolated_drive',
    ]);
    expect(stoppedStatus.stdout).toContain('Stopped');

    await writeFile(join(harness.paths.vaultA, 'after_stop.txt'), 'Still syncing smoke_drive');
    const rootHash = await waitForStoreRoot(harness.storePort, 'smoke_drive');
    expect(rootHash).toBeTruthy();

    // 4. Restart isolated_drive.
    await harness.cli(harness.mainSocketPath, ['start', 'isolated_drive']);
    const restartedStatus = await harness.cli(harness.mainSocketPath, [
      'status',
      '--drive-id',
      'isolated_drive',
    ]);
    expect(restartedStatus.stdout).not.toContain('Stopped');

    // 5. Add a third drive against a fresh directory and confirm it syncs.
    const vaultD = join(harness.paths.sandboxDir, 'vault_d');
    await mkdir(vaultD, { recursive: true });
    await writeFile(join(vaultD, 'seed.txt'), 'Seed content for the third drive');

    await harness.cli(harness.mainSocketPath, [
      'add',
      'third-drive',
      '--path',
      vaultD,
    ]);

    const thirdRoot = await waitForStoreRoot(harness.storePort, 'third-drive');
    expect(thirdRoot).toBeTruthy();

    // 6. Remove it and confirm the config file no longer has the entry.
    await harness.cli(harness.mainSocketPath, ['remove', 'third-drive']);

    const configContent = await readFile(harness.paths.configPath, 'utf-8');
    expect(configContent).not.toContain('third-drive');

    await rm(vaultD, { recursive: true, force: true });
  });
});
