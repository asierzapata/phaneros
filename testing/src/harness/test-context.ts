import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdir, writeFile, rm } from 'node:fs/promises';
import { createWriteStream } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto';
import type { ManagedProcess, TestHarnessPaths, HarnessOptions, DaemonHandle } from './types.js';
import { getFreePort, sleep, dumpLogs, waitForServer, waitForDaemonReady } from './utils.js';

const execFileAsync = promisify(execFile);

/**
 * A unix socket path short enough to stay under macOS's ~104-byte
 * `sun_path` limit — `os.tmpdir()`-based sandbox directories are often too
 * deep for that, so daemon sockets live directly under `/tmp` instead.
 */
function shortSocketPath(name: string): string {
  return join('/tmp', `phaneros-e2e-${randomUUID().slice(0, 8)}-${name}.sock`);
}

export class TestHarness {
  public readonly rootDir: string;
  public readonly storeBin: string;
  public readonly cliBin: string;
  public readonly daemonBin: string;
  public readonly storePort: number;
  public readonly storeUrl: string;
  public readonly paths: TestHarnessPaths;
  /** Socket for a daemon started directly against `paths.configPath` (multi-drive, single daemon). */
  public readonly mainSocketPath: string;
  public readonly managedProcesses: ManagedProcess[] = [];

  private constructor(
    rootDir: string,
    storePort: number,
    paths: TestHarnessPaths,
    mainSocketPath: string
  ) {
    this.rootDir = rootDir;
    this.storeBin = join(rootDir, 'target', 'debug', 'phaneros-store');
    this.cliBin = join(rootDir, 'target', 'debug', 'phaneros');
    this.daemonBin = join(rootDir, 'target', 'debug', 'phanerosd');
    this.storePort = storePort;
    this.storeUrl = `http://127.0.0.1:${storePort}`;
    this.paths = paths;
    this.mainSocketPath = mainSocketPath;
  }

  public static async create(options: HarnessOptions = {}): Promise<TestHarness> {
    const rootDir = resolve(__dirname, '..', '..', '..');
    const storePort = options.storePort ?? (await getFreePort());
    const storeUrl = `http://127.0.0.1:${storePort}`;

    const sandboxDir = join(tmpdir(), `phaneros_smoke_${randomUUID()}`);
    const logsDir = join(sandboxDir, 'logs');
    const storeDataDir = join(sandboxDir, 'store_data');
    const storeBlobsDir = join(sandboxDir, 'store_blobs');
    const dbPath = join(storeDataDir, 'phaneros_test.db');
    const vaultA = join(sandboxDir, 'vault_a');
    const vaultB = join(sandboxDir, 'vault_b');
    const vaultC = join(sandboxDir, 'vault_c');
    const configPath = join(sandboxDir, 'config.toml');
    const mainSocketPath = shortSocketPath('main');

    await mkdir(logsDir, { recursive: true });
    await mkdir(storeDataDir, { recursive: true });
    await mkdir(storeBlobsDir, { recursive: true });
    await mkdir(vaultA, { recursive: true });
    await mkdir(vaultB, { recursive: true });
    await mkdir(vaultC, { recursive: true });

    // Declares both drives under one daemon config; used directly by tests
    // that spin up a single multi-drive daemon (e.g. the control-plane
    // suite). Tests simulating separate devices instead use spawnDaemon,
    // which writes its own single-drive config per simulated device.
    const configContent = `
[daemon]
store_url = "${storeUrl}"
log_level = "info"
enable_telemetry = true
ipc_socket = "${mainSocketPath}"

[drives.smoke_drive]
path = "${vaultA}"
token = "smoke-test-token"
enabled = true

[drives.isolated_drive]
path = "${vaultC}"
token = "smoke-test-token"
enabled = true
`;
    await writeFile(configPath, configContent);

    const paths: TestHarnessPaths = {
      sandboxDir,
      logsDir,
      storeDataDir,
      storeBlobsDir,
      dbPath,
      configPath,
      vaultA,
      vaultB,
      vaultC,
    };

    return new TestHarness(rootDir, storePort, paths, mainSocketPath);
  }

  public spawnManagedProcess(
    name: string,
    command: string,
    args: string[],
    options: { env?: Record<string, string>; cwd?: string } = {}
  ): ManagedProcess {
    const logFile = join(this.paths.logsDir, `${name}.log`);
    const logStream = createWriteStream(logFile, { flags: 'a' });

    const proc = spawn(command, args, {
      env: options.env ? { ...process.env, ...options.env } : process.env,
      cwd: options.cwd,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    proc.stdout?.pipe(logStream);
    proc.stderr?.pipe(logStream);

    const managed: ManagedProcess = { name, proc, logFile, logStream };
    this.managedProcesses.push(managed);
    return managed;
  }

  public async spawnStore(name = 'store'): Promise<ManagedProcess> {
    const storeProc = this.spawnManagedProcess(name, this.storeBin, [], {
      env: {
        PORT: String(this.storePort),
        HOST: '127.0.0.1',
        DATABASE_PATH: this.paths.dbPath,
        BLOB_STORAGE_PATH: this.paths.storeBlobsDir,
        PUBLIC_URL: this.storeUrl,
      },
    });

    await waitForServer(this.storePort);
    return storeProc;
  }

  /**
   * Spawns a `phanerosd` instance simulating one device syncing one drive:
   * its own single-drive config and its own IPC socket, so several of these
   * can run side by side against the same remote drive_id (the multi-device
   * sync scenarios most tests exercise) without stepping on each other.
   */
  public async spawnDaemon(
    name: string,
    vaultPath: string,
    driveId = 'smoke_drive'
  ): Promise<DaemonHandle> {
    const configPath = join(this.paths.sandboxDir, `${name}.config.toml`);
    const socketPath = shortSocketPath(name);

    const configContent = `
[daemon]
store_url = "${this.storeUrl}"
log_level = "info"
enable_telemetry = true
ipc_socket = "${socketPath}"

[drives.${driveId}]
path = "${vaultPath}"
token = "smoke-test-token"
enabled = true
`;
    await writeFile(configPath, configContent);

    const proc = this.spawnManagedProcess(name, this.daemonBin, ['--config', configPath], {
      env: {
        HOME: this.paths.sandboxDir,
        XDG_CONFIG_HOME: this.paths.sandboxDir,
      },
    });
    await waitForDaemonReady(this.cliBin, socketPath);

    return { proc, socketPath, configPath, driveId };
  }

  /** Runs the real `phaneros` CLI against a specific daemon's socket. */
  public async cli(
    socketPath: string,
    args: string[],
    options: { env?: Record<string, string> } = {}
  ): Promise<{ stdout: string; stderr: string }> {
    return execFileAsync(this.cliBin, ['--socket', socketPath, ...args], {
      env: options.env ? { ...process.env, ...options.env } : process.env,
    });
  }

  public async killManagedProcess(item: ManagedProcess): Promise<void> {
    const { proc, logStream } = item;
    if (proc.exitCode === null && proc.signalCode === null) {
      proc.kill('SIGTERM');
      const startTime = Date.now();
      while (Date.now() - startTime < 1000) {
        if (proc.exitCode !== null || proc.signalCode !== null) break;
        await sleep(50);
      }
      if (proc.exitCode === null && proc.signalCode === null) {
        proc.kill('SIGKILL');
      }
    }
    logStream.end();
  }

  public async dumpLogsOnFailure(): Promise<void> {
    await dumpLogs(this.managedProcesses);
  }

  public async teardown(): Promise<void> {
    for (const item of this.managedProcesses) {
      await this.killManagedProcess(item);
    }
    await rm(this.paths.sandboxDir, { recursive: true, force: true });
  }
}
