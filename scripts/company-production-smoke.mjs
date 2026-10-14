import { spawn } from 'node:child_process';
import { mkdtemp, access, rm } from 'node:fs/promises';
import { join, resolve, dirname, basename } from 'node:path';
import { tmpdir } from 'node:os';
import assert from 'node:assert/strict';

const root = await mkdtemp(join(tmpdir(), 'company-release-guard-'));
const profile = join(root, 'profile'); const data = join(root, 'data');
const child = spawn(resolve('src-tauri/target/release/skills-manager.exe'), [], {
  cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
  env: { ...process.env, COMPANY_STORE_MODE: 'mock', COMPANY_STORE_BASE_URL: 'http://127.0.0.1:4318', COMPANY_PROFILE_ROOT: profile, COMPANY_DATA_ROOT: data },
});
let output = '';
child.stdout.on('data', chunk => output += chunk); child.stderr.on('data', chunk => output += chunk);
const timeout = setTimeout(() => child.kill(), 15000);
try {
  const code = await new Promise((done, reject) => { child.once('error', reject); child.once('exit', done); });
  assert.notEqual(code, 0, 'Production executable must reject mock/debug overrides');
  assert.match(output, /debug\/mock|không nhận cấu hình debug/);
  await assert.rejects(access(profile)); await assert.rejects(access(data));
  console.log('PASS actual production executable: rejects mock/debug configuration before writing profile/data');
} finally {
  clearTimeout(timeout);
  assert.equal(dirname(resolve(root)), resolve(tmpdir())); assert(basename(root).startsWith('company-release-guard-'));
  await rm(root, { recursive: true, force: true });
}
