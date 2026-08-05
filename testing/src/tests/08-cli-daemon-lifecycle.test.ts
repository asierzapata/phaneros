import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { writeFile, mkdir, rm } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto';
import { TestHarness } from '../harness/test-context.js';
import { waitForCondition, waitForStoreRoot } from '../harness/utils.js';

const execFileAsync = promisify(execFile);

describe('CLI daemon lifecycle (start/stop/status/activity)', () => {
  let harness: TestHarness;
  let sandboxDir: string;
  let configPath: string;
  let socketPath: string;
  let cliEnv: Record<string, string>;
  let stopped = false;

  beforeEach(async () => {
    harness = await TestHarness.create();
    sandboxDir = join(tmpdir(), `phaneros_cli_lifecycle_${randomUUID()}`);
    await mkdir(sandboxDir, { recursive: true });
    configPath = join(sandboxDir, 'config.toml');
    socketPath = join('/tmp', `phaneros-e2e-${randomUUID().slice(0, 8)}-cli-lifecycle.sock`);
    // Put the freshly built `phanerosd` on $PATH so the CLI's own `$PATH`
    // lookup (the same code path a real end user relies on) resolves it.
    cliEnv = {
      ...process.env,
      PATH: `${dirname(harness.daemonBin)}:${process.env.PATH}`,
    };
    stopped = false;
  });

  afterEach(async (ctx) => {
    if (ctx.task?.result?.state === 'fail') {
      await harness.dumpLogsOnFailure();
    }
    // The daemon here was spawned by the CLI itself, not tracked in
    // harness.managedProcesses, so it won't be cleaned up by
    // harness.teardown(). Always try to stop it, even if assertions threw.
    if (!stopped) {
      try {
        await execFileAsync(harness.cliBin, ['--socket', socketPath, 'daemon', 'stop'], {
          env: cliEnv,
        });
      } catch {
        // Already stopped or never started; ignore.
      }
    }
    await harness.teardown();
    await rm(sandboxDir, { recursive: true, force: true });
  });

  it('starts, reports status, records activity, and stops via the CLI', async () => {
    // 1. A sandboxed single-drive config the CLI-spawned daemon will read.
    await harness.spawnStore();
    const vaultPath = join(sandboxDir, 'vault');
    await mkdir(vaultPath, { recursive: true });
    await writeFile(join(vaultPath, 'hello.txt'), 'Hello from the CLI-managed daemon');

    const configContent = `
[daemon]
store_url = "${harness.storeUrl}"
log_level = "info"
enable_telemetry = true
ipc_socket = "${socketPath}"

[drives.cli_drive]
path = "${vaultPath}"
token = "smoke-test-token"
enabled = true
`;
    await writeFile(configPath, configContent);

    // 2. Before starting, the daemon is unreachable.
    const beforeStatus = await execFileAsync(
      harness.cliBin,
      ['--socket', socketPath, 'daemon', 'status'],
      { env: cliEnv }
    );
    expect(beforeStatus.stdout).toContain('Unreachable');

    // 3. `daemon start` spawns phanerosd itself (via $PATH), not the harness.
    const startResult = await execFileAsync(
      harness.cliBin,
      ['--socket', socketPath, 'daemon', 'start', '--config', configPath],
      { env: cliEnv }
    );
    expect(startResult.stdout).toContain('Started phanerosd');

    await waitForCondition(
      async () => {
        try {
          await execFileAsync(harness.cliBin, ['--socket', socketPath, 'daemon', 'ping'], {
            env: cliEnv,
          });
          return true;
        } catch {
          return false;
        }
      },
      10000,
      150,
      'CLI-started daemon to become reachable'
    );

    const afterStatus = await execFileAsync(
      harness.cliBin,
      ['--socket', socketPath, 'daemon', 'status'],
      { env: cliEnv }
    );
    expect(afterStatus.stdout).toContain('Reachable');

    // 4. Let a real sync happen so `activity` has a session to report.
    const rootHash = await waitForStoreRoot(harness.storePort, 'cli_drive');
    expect(rootHash).toBeTruthy();

    const activityJson = await execFileAsync(
      harness.cliBin,
      ['--socket', socketPath, 'activity', '--drive-id', 'cli_drive', '--json'],
      { env: cliEnv }
    );
    const sessions = JSON.parse(activityJson.stdout);
    expect(Array.isArray(sessions)).toBe(true);
    expect(sessions.length).toBeGreaterThanOrEqual(1);
    expect(sessions[0]).toHaveProperty('drive_id', 'cli_drive');

    const activityTable = await execFileAsync(
      harness.cliBin,
      ['--socket', socketPath, 'activity', '--drive-id', 'cli_drive'],
      { env: cliEnv }
    );
    expect(activityTable.stdout).toContain('cli_drive');

    // 5. `daemon stop` gracefully shuts down the daemon; a subsequent ping fails.
    await execFileAsync(harness.cliBin, ['--socket', socketPath, 'daemon', 'stop'], {
      env: cliEnv,
    });
    stopped = true;

    await waitForCondition(
      async () => {
        try {
          await execFileAsync(harness.cliBin, ['--socket', socketPath, 'daemon', 'ping'], {
            env: cliEnv,
          });
          return false;
        } catch {
          return true;
        }
      },
      10000,
      150,
      'daemon to become unreachable after stop'
    );

    const finalStatus = await execFileAsync(
      harness.cliBin,
      ['--socket', socketPath, 'daemon', 'status'],
      { env: cliEnv }
    );
    expect(finalStatus.stdout).toContain('Unreachable');
  });
});

// This suite touches real OS state (`~/Library/LaunchAgents`, `launchctl`)
// instead of the sandboxed harness directories, since login-item
// registration is inherently per-user/global. Kept isolated in its own
// `describe` block and always torn down in `afterEach`, even on failure, so
// a broken assertion never leaves a stray LaunchAgent registered on the
// dev/CI machine.
describe.skipIf(process.platform !== 'darwin')('CLI daemon login item (macOS only)', () => {
  const label = 'com.asierzapata.phaneros-cli.phanerosd';
  let harness: TestHarness;

  beforeEach(async () => {
    harness = await TestHarness.create();
  });

  afterEach(async (ctx) => {
    if (ctx.task?.result?.state === 'fail') {
      await harness.dumpLogsOnFailure();
    }
    // Always attempt uninstall, regardless of test outcome.
    await execFileAsync(harness.cliBin, ['daemon', 'uninstall']).catch(() => {});
    await harness.teardown();
  });

  it('installs and uninstalls the login item', async () => {
    const cliEnv = {
      ...process.env,
      PATH: `${dirname(harness.daemonBin)}:${process.env.PATH}`,
    };

    const installResult = await execFileAsync(harness.cliBin, ['daemon', 'install'], {
      env: cliEnv,
    });
    expect(installResult.stdout).toContain('Registered');

    const { stdout: listedAfterInstall } = await execFileAsync('launchctl', ['list']);
    expect(listedAfterInstall).toContain(label);

    const statusResult = await execFileAsync(harness.cliBin, [
      '--socket',
      '/tmp/nonexistent-for-status-check.sock',
      'daemon',
      'status',
    ]);
    expect(statusResult.stdout).toContain('installed');

    const uninstallResult = await execFileAsync(harness.cliBin, ['daemon', 'uninstall']);
    expect(uninstallResult.stdout).toContain('Unregistered');

    const { stdout: listedAfterUninstall } = await execFileAsync('launchctl', ['list']).catch(
      () => ({ stdout: '' })
    );
    expect(listedAfterUninstall).not.toContain(label);
  });
});
