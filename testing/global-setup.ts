import { execSync } from 'node:child_process';
import { join } from 'node:path';

export default async function setup() {
  const rootDir = join(__dirname, '..');
  console.log('\n🔨 [Global Setup] Building Rust workspace binaries with Cargo...');
  execSync('cargo build --workspace', { stdio: 'inherit', cwd: rootDir });
  console.log('✅ [Global Setup] Workspace binaries built successfully.');
}
