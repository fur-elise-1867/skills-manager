import { createMockStore } from './company-mock-store.mjs';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createInterface } from 'node:readline';

const pack = fileURLToPath(new URL('../', import.meta.url));
const args = process.argv.slice(2);
let serverOnly = false, publishUpdate = false, port = 4319;
let root = join(process.env.LOCALAPPDATA || homedir(), 'CompanySkillsManagerAiproAcceptance142');
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--server-only') serverOnly = true;
  else if (args[i] === '--publish-update') publishUpdate = true;
  else if (args[i] === '--port' && args[i + 1] !== undefined) port = Number(args[++i]);
  else if (args[i] === '--data-root' && args[i + 1]) root = resolve(args[++i]);
  else throw new Error(`Tham số không hợp lệ: ${args[i]}`);
}
if (!Number.isInteger(port) || port < 0 || port > 65535) throw new Error('Port không hợp lệ.');
const profile = join(root, 'profile');
const revisionFile = join(root, 'store-revision.txt');
await mkdir(root, { recursive: true });
if (publishUpdate) {
  await writeFile(revisionFile, '2');
  console.log(`Đã công bố gói Jira v2 GIẢ LẬP. Trong app chọn Skill đã cài > Kiểm tra gói mới.\nDemo: ${root}`);
} else {
  await Promise.all([
    mkdir(join(profile, '.aipro'), { recursive: true }),
    mkdir(join(root, 'data'), { recursive: true }),
    mkdir(join(root, 'projects', 'Project A'), { recursive: true }),
    mkdir(join(root, 'projects', 'Project B'), { recursive: true }),
  ]);
  async function createOnce(path, content) {
    try { await writeFile(path, content, { flag: 'wx' }); }
    catch (error) { if (error.code !== 'EEXIST') throw error; }
  }
  await createOnce(join(profile, '.aipro', 'mcp.json'), '{"mcpServers":{}}');
  await createOnce(revisionFile, '1');
  const server = createMockStore({ revisionFile });
  let child, input, stopping = false;
  function stop() {
    if (stopping) return;
    stopping = true;
    input?.close();
    if (child && child.exitCode === null) child.kill();
    server.close();
    server.closeAllConnections();
  }
  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);
  await new Promise((done, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', done);
  }).catch(error => {
    console.error(`Không mở được Store giả lập: ${error.message}. Nếu demo đang chạy, dùng cửa sổ đó; không mở hai bản cùng lúc.`);
    throw error;
  });
  const url = `http://127.0.0.1:${server.address().port}`;
  console.log(`SERVER_READY ${url}\nDEMO OFFLINE — DỮ LIỆU GIẢ LẬP\nTài khoản: employee01 / demo-password\nTài khoản thứ hai: employee02 / demo-password\nDemo profile: ${profile}\nDự án mẫu: ${join(root, 'projects')}\nGiữ cửa sổ này mở. Ctrl+C để dừng demo.`);
  if (!serverOnly) {
    function startApp() {
      if (stopping || (child && child.exitCode === null)) return;
      child = spawn(join(pack, 'app', 'skills-manager.exe'), [], {
        cwd: join(pack, 'app'), windowsHide: true, stdio: 'inherit',
        env: { ...process.env, COMPANY_STORE_MODE: 'mock', COMPANY_STORE_BASE_URL: url,
          COMPANY_PROFILE_ROOT: profile, COMPANY_DATA_ROOT: join(root, 'data') },
      });
      child.once('spawn', () => console.log(`APP_STARTED ${child.pid}`));
      child.once('error', error => { console.error(`Không mở được app: ${error.message}`); process.exitCode = 1; stop(); });
      child.once('exit', code => {
        if (!stopping) console.log(`App đã đóng (mã ${code}). Nhấn Enter để mở lại; Ctrl+C để dừng toàn bộ demo.`);
      });
    }
    input = createInterface({ input: process.stdin, output: process.stdout });
    input.on('line', startApp);
    input.on('close', stop);
    startApp();
  }
}

