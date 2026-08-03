import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globalSetup: ['./global-setup.ts'],
    testTimeout: 45000,
    hookTimeout: 45000,
    fileParallelism: true,
    maxConcurrency: 4,
    include: ['src/tests/**/*.test.ts'],
  },
});
