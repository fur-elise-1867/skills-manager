import { chromium } from 'playwright-core';
import { createMockStore } from './company-mock-store.mjs';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdtemp, mkdir, readFile, writeFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import assert from 'node:assert/strict';

const root = await mkdtemp(join(tmpdir(), 'company-native-l4-l5-'));
const profile = join(root, 'profile'); const data = join(root, 'data');
const projectA = join(root, 'Project A'); const projectB = join(root, 'Project B');
await Promise.all([mkdir(join(profile, '.aipro'), { recursive: true }), mkdir(projectA), mkdir(projectB)]);
const mcp = join(profile, '.aipro', 'mcp.json'); await writeFile(mcp, '{"mcpServers":{}}');
const mcpBefore = await readFile(mcp, 'utf8');
const server = createMockStore(); await new Promise(done => server.listen(0, '127.0.0.1', done));
const portListener = createServer(); await new Promise(done => portListener.listen(0, '127.0.0.1', done));
const port = portListener.address().port; await new Promise(done => portListener.close(done));
const env = { ...process.env, COMPANY_STORE_MODE: 'mock', COMPANY_STORE_BASE_URL: `http://127.0.0.1:${server.address().port}`, COMPANY_PROFILE_ROOT: profile, COMPANY_DATA_ROOT: data, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}` };
let child; let browser; let page;
async function start() {
  child = spawn(resolve('src-tauri/target/debug/skills-manager.exe'), [], { env, windowsHide: true, stdio: 'ignore' });
  for (let i = 0; i < 100; i++) {
    assert(child.exitCode === null, 'Owned app exited; another Company app instance may already be running');
    try { browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`); break; }
    catch { await new Promise(done => setTimeout(done, 200)); }
  }
  assert(browser, 'Native WebView2 unavailable');
  for (let i = 0; i < 50; i++) {
    page = browser.contexts().flatMap(c => c.pages()).find(p => !p.url().startsWith('devtools:'));
    if (page) break; await new Promise(done => setTimeout(done, 100));
  }
  assert(page, 'Native app page missing'); page.setDefaultTimeout(20_000);
  await page.getByText('Dữ liệu giả lập', { exact: true }).waitFor();
}
async function stop() {
  if (browser) { await browser.close().catch(() => {}); browser = undefined; }
  if (child && child.exitCode === null) { const exited = new Promise(done => child.once('exit', done)); child.kill(); await exited; }
  child = undefined; page = undefined;
}
async function install(projectName) {
  await page.getByRole('button', { name: 'Store', exact: true }).click();
  await page.getByPlaceholder('Tìm theo tên hoặc công dụng skill…').fill('jira');
  await page.getByRole('button', { name: 'Xem chi tiết Tổng hợp tiến độ Jira', exact: true }).click();
  await page.getByLabel('Phạm vi cài', { exact: true }).selectOption({ label: projectName ? `Dự án: ${projectName}` : 'AIPRO Global' });
  await page.getByRole('button', { name: projectName ? 'Cài vào AIPRO Dự án' : 'Cài vào AIPRO Global', exact: true }).click();
  await page.getByRole('button', { name: 'Bỏ qua và cài tiếp', exact: true }).click();
  await page.getByText('Cài xong; đã bỏ qua cảnh báo MCP của lần này.', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Hoàn tất', exact: true }).click();
}
try {
  await start();
  await page.getByLabel('Tài khoản', { exact: true }).fill('employee01');
  await page.getByLabel('Mật khẩu', { exact: true }).fill('demo-password');
  await page.getByRole('button', { name: 'Đăng nhập', exact: true }).click();
  await page.getByText('Nhân viên 01', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Dự án', exact: true }).click();
  for (const path of [projectA, projectB]) {
    await page.getByLabel('Thư mục dự án', { exact: true }).fill(path);
    await page.getByRole('button', { name: 'Thêm dự án', exact: true }).click();
    await page.getByRole('heading', { name: basename(path), exact: true }).waitFor();
    await assert.rejects(readFile(join(path, '.aipro/skills/jira-project-summary/SKILL.md')));
  }
  await install(); await install('Project A'); await install('Project B');
  const globalTarget = join(profile, '.aipro/skills/jira-project-summary');
  const aTarget = join(projectA, '.aipro/skills/jira-project-summary');
  const bTarget = join(projectB, '.aipro/skills/jira-project-summary');
  const originalGlobal = await readFile(join(globalTarget, 'SKILL.md'));
  const originalB = await readFile(join(bTarget, 'SKILL.md'));
  await writeFile(join(aTarget, 'SKILL.md'), 'Native project customization');
  server.setRevision(2);
  await page.getByRole('button', { name: 'Skill đã cài', exact: true }).click();
  await page.getByRole('button', { name: 'Làm mới nội dung', exact: true }).click();
  await page.getByText('Đã tùy chỉnh', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Kiểm tra gói mới', exact: true }).click();
  await page.getByText('Có gói mới', { exact: true }).first().waitFor();
  await page.getByLabel('Chọn Tổng hợp tiến độ Jira tại Dự án: Project A', { exact: true }).check();
  await page.getByRole('button', { name: 'Xem cập nhật các đích đã chọn (1)', exact: true }).click();
  await page.getByRole('button', { name: 'Bỏ qua và xem thay đổi', exact: true }).click();
  await page.getByText('SKILL.md', { exact: true }).waitFor();
  assert.equal(await readFile(join(aTarget, 'SKILL.md'), 'utf8'), 'Native project customization');
  assert.equal(await page.getByRole('button', { name: 'Backup và cập nhật', exact: true }).isEnabled(), false);
  await page.getByLabel('Tôi đồng ý thay bản đã tùy chỉnh bằng gói Store', { exact: true }).check();
  await page.getByRole('button', { name: 'Backup và cập nhật', exact: true }).click();
  await page.getByText('Đã cập nhật đích này; bản trước đã được lưu.', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Hoàn tất', exact: true }).click();
  assert.match(await readFile(join(aTarget, 'SKILL.md'), 'utf8'), /Jira project summary v2/);
  assert.deepEqual(await readFile(join(globalTarget, 'SKILL.md')), originalGlobal);
  assert.deepEqual(await readFile(join(bTarget, 'SKILL.md')), originalB);
  const installationDirs = await readdir(join(data, 'backups')); assert.equal(installationDirs.length, 1);
  const backupDirs = await readdir(join(data, 'backups', installationDirs[0])); assert.equal(backupDirs.length, 1);
  const backup = JSON.parse(await readFile(join(data, 'backups', installationDirs[0], backupDirs[0], 'metadata.json'), 'utf8'));
  assert.equal(await readFile(join(backup.path, 'SKILL.md'), 'utf8'), 'Native project customization');
  assert.equal(await readFile(mcp, 'utf8'), mcpBefore);
  await stop(); await start();
  await page.getByText('Nhân viên 01', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Dự án', exact: true }).click();
  await page.getByRole('heading', { name: 'Project A', exact: true }).waitFor();
  await page.getByRole('heading', { name: 'Project B', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Skill đã cài', exact: true }).click();
  assert.equal(await page.getByRole('heading', { name: 'Tổng hợp tiến độ Jira', exact: true }).count(), 3);
  const screenshot = process.env.COMPANY_SMOKE_SCREENSHOT;
  if (screenshot) { await mkdir(dirname(resolve(screenshot)), { recursive: true }); await page.screenshot({ path: screenshot, fullPage: true }); }
  await page.getByRole('button', { name: 'Đăng xuất', exact: true }).click();
  console.log('PASS native L4/L5: projects, independent copies, customization, update preview/backup/apply, restart and unchanged MCP');
} finally {
  if (page) await page.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke('company_logout', {}); } catch { /* owned mock credentials only */ } }).catch(() => {});
  await stop(); await new Promise(done => server.close(done));
  assert.equal(dirname(resolve(root)), resolve(tmpdir())); assert(basename(root).startsWith('company-native-l4-l5-'));
  await rm(root, { recursive: true, force: true });
}
