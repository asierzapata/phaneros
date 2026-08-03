import { spawn } from 'node:child_process';
import { mkdir, writeFile, rm } from 'node:fs/promises';
import { createWriteStream } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto';
import type { ManagedProcess, TestHarnessPaths, HarnessOptions } from './types.js';
import { getFreePort, sleep, dumpLogs, waitForServer } from './utils.js';

export class TestHarness {
  public readonly rootDir: string;
  public readonly storeBin: string;
  public readonly cliBin: string;
  public readonly storePort: number;
  public readonly storeUrl: string;
  public readonly paths: TestHarnessPaths;
  public readonly managedProcesses: ManagedProcess[] = [];

  private constructor(rootDir: string, storePort: number, paths: TestHarnessPaths) {
    this.rootDir = rootDir;
    this.storeBin = join(rootDir, 'target', 'debug', 'phaneros-store');
    this.cliBin = join(rootDir, 'target', 'debug', 'phaneros');
    this.storePort = storePort;
    this.storeUrl = `http://127.0.0.1:${storePort}`;
    this.paths = paths;
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

    await mkdir(logsDir, { recursive: true });
    await mkdir(storeDataDir, { recursive: true });
    await mkdir(storeBlobsDir, { recursive: true });
    await mkdir(vaultA, { recursive: true });
    await mkdir(vaultB, { recursive: true });
    await mkdir(vaultC, { recursive: true });

    const configContent = `
[daemon]
store_url = "${storeUrl}"
log_level = "info"

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

    return new TestHarness(rootDir, storePort, paths);
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

  public spawnClient(name: string, vaultPath: string, driveId = 'smoke_drive'): ManagedProcess {
    return this.spawnManagedProcess(
      name,
      this.cliBin,
      [
        '--config',
        this.paths.configPath,
        vaultPath,
        '--drive-id',
        driveId,
        '--store-url',
        this.storeUrl,
      ],
      {
        env: {
          HOME: this.paths.sandboxDir,
          XDG_CONFIG_HOME: this.paths.sandboxDir,
        },
      }
    );
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
