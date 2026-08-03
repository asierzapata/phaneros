import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { TestHarness } from '../harness/test-context.js';
import {
  waitForStoreRoot,
  waitForFileContent,
  waitForBufferMatch,
} from '../harness/utils.js';

describe('Initial File Synchronization & Ignore Rules', () => {
  let harness: TestHarness;

  beforeEach(async () => {
    harness = await TestHarness.create();
  });

  afterEach(async () => {
    await harness.teardown();
  });

  it('synchronizes text, JSON, binary files, and respects .phanerosignore', async () => {
    // 1. Seed vault_a
    const randomBinaryData = randomBytes(1024 * 64);
    await writeFile(join(harness.paths.vaultA, 'hello.txt'), 'Hello, Phaneros End-to-End!');
    await writeFile(join(harness.paths.vaultA, 'data.json'), JSON.stringify({ status: 'success', phase: 2 }));
    await writeFile(join(harness.paths.vaultA, 'binary.dat'), randomBinaryData);
    await writeFile(join(harness.paths.vaultA, '.phanerosignore'), '*.tmp\nignored_dir/\n');
    await writeFile(join(harness.paths.vaultA, 'scratch.tmp'), 'This file should be ignored!');

    const subDirA = join(harness.paths.vaultA, 'subfolder');
    await mkdir(subDirA, { recursive: true });
    await writeFile(join(subDirA, 'notes.md'), '# Smoke Test Notes');

    // 2. Start store server & Client A
    await harness.spawnStore();
    harness.spawnClient('client_a', harness.paths.vaultA, 'smoke_drive');

    const initialRootHash = await waitForStoreRoot(harness.storePort, 'smoke_drive');
    expect(initialRootHash).toBeTruthy();

    // 3. Start Client B for vault_b
    harness.spawnClient('client_b', harness.paths.vaultB, 'smoke_drive');

    // 4. Assertions on vault_b
    const helloContentB = await waitForFileContent(
      join(harness.paths.vaultB, 'hello.txt'),
      'Hello, Phaneros End-to-End!'
    );
    expect(helloContentB).toBe('Hello, Phaneros End-to-End!');

    const dataContentB = await waitForFileContent(
      join(harness.paths.vaultB, 'data.json'),
      JSON.stringify({ status: 'success', phase: 2 })
    );
    expect(JSON.parse(dataContentB).status).toBe('success');

    const notesContentB = await waitForFileContent(
      join(harness.paths.vaultB, 'subfolder', 'notes.md'),
      '# Smoke Test Notes'
    );
    expect(notesContentB).toBe('# Smoke Test Notes');

    const binaryBufferB = await waitForBufferMatch(
      join(harness.paths.vaultB, 'binary.dat'),
      randomBinaryData
    );
    expect(binaryBufferB.equals(randomBinaryData)).toBe(true);

    let scratchExistsInB = false;
    try {
      await readFile(join(harness.paths.vaultB, 'scratch.tmp'));
      scratchExistsInB = true;
    } catch {
      scratchExistsInB = false;
    }
    expect(scratchExistsInB).toBe(false);
  });
});
