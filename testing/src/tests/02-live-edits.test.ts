import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, unlink } from 'node:fs/promises';
import { join } from 'node:path';
import { TestHarness } from '../harness/test-context.js';
import {
  waitForStoreRoot,
  waitForFileContent,
  waitForFileAbsence,
} from '../harness/utils.js';

describe('Live File Operations & Synchronization', () => {
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

  it('handles live edits, deletions, and directory creations across clients', async () => {
    // 1. Initial setup
    await writeFile(join(harness.paths.vaultA, 'hello.txt'), 'Hello Initial');
    await writeFile(join(harness.paths.vaultA, 'data.json'), JSON.stringify({ item: 1 }));

    await harness.spawnStore();
    harness.spawnClient('client_a', harness.paths.vaultA, 'smoke_drive');
    await waitForStoreRoot(harness.storePort, 'smoke_drive');

    harness.spawnClient('client_b', harness.paths.vaultB, 'smoke_drive');
    await waitForFileContent(join(harness.paths.vaultB, 'hello.txt'), 'Hello Initial');

    // 2. Live modification test
    await writeFile(join(harness.paths.vaultA, 'hello.txt'), 'Hello Initial [MODIFIED]');
    const updatedHelloB = await waitForFileContent(
      join(harness.paths.vaultB, 'hello.txt'),
      'Hello Initial [MODIFIED]'
    );
    expect(updatedHelloB).toBe('Hello Initial [MODIFIED]');

    // 3. Live deletion test
    await unlink(join(harness.paths.vaultA, 'data.json'));
    await waitForFileAbsence(join(harness.paths.vaultB, 'data.json'));

    // 4. Live directory & nested file creation test
    const newDirA = join(harness.paths.vaultA, 'new_dir');
    await mkdir(newDirA, { recursive: true });
    await writeFile(join(newDirA, 'nested.txt'), 'Nested file content');

    const nestedContentB = await waitForFileContent(
      join(harness.paths.vaultB, 'new_dir', 'nested.txt'),
      'Nested file content'
    );
    expect(nestedContentB).toBe('Nested file content');
  });
});
