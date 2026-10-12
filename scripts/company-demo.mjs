import { createMockStore } from './company-mock-store.mjs';
import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { homedir } from 'node:os';

const root = join(process.env.LOCALAPPDATA || homedir(), 'CompanySkillsManagerDemo');
const profile = join(root, 'profile');
mkdirSync(join(profile, '.aipro'), { recursive: true });
const mcp = join(profile, '.aipro', 'mcp.json');
if (!existsSync(mcp)) writeFileSync(mcp, JSON.stringify({ mcpServers: {} }, null, 2));
const server = createMockStore();
server.on('error', error => { console.error(`Không mở được Store giả lập: ${error.message}`); process.exitCode = 1; });
server.listen(4318, '127.0.0.1', () => {
  console.log(`Demo profile: ${profile}\nTài khoản: employee01 / demo-password`);
  const command = process.platform === 'win32' ? 'cmd.exe' : 'npm';
  const args = process.platform === 'win32' ? ['/d', '/s', '/c', 'npm run tauri:dev'] : ['run', 'tauri:dev'];
  const sep = process.platform === 'win32' ? ';' : ':';
  const extraPaths = [join(process.env.CARGO_HOME || join(homedir(), '.cargo'), 'bin')];
  if (process.platform === 'win32') {
    try { const git = execFileSync('where.exe', ['git'], { encoding: 'utf8', windowsHide: true }).trim().split(/\r?\n/)[0]; const perl = join(dirname(dirname(git)), 'usr', 'bin'); if (existsSync(perl)) extraPaths.push(perl); } catch { /* Git is optional when the toolchain already has Perl. */ }
  }
  const env = { ...process.env, OPENSSL_NO_ASM: '1', COMPANY_STORE_MODE: 'mock', COMPANY_STORE_BASE_URL: 'http://127.0.0.1:4318', COMPANY_PROFILE_ROOT: profile, COMPANY_DATA_ROOT: join(root, 'data') };
  const pathKey = Object.keys(env).find(key => key.toLowerCase() === 'path') || 'PATH';
  env[pathKey] = [...extraPaths, env[pathKey] || ''].join(sep);
  const child = spawn(command, args, { env, stdio: 'inherit', windowsHide: true });
  child.on('exit', code => { server.close(); process.exitCode = code || 0; });
  process.on('SIGINT', () => { child.kill(); server.close(); });
  process.on('SIGTERM', () => { child.kill(); server.close(); });
});
