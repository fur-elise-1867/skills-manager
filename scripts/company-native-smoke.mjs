import { chromium } from 'playwright-core';
import { createMockStore } from './company-mock-store.mjs';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';
import assert from 'node:assert/strict';

const root = await mkdtemp(join(tmpdir(), 'company-native-l1-l3-'));
const profile = join(root, 'profile');
await mkdir(join(profile, '.aipro'), { recursive: true });
const mcpFile = join(profile, '.aipro', 'mcp.json');
await writeFile(mcpFile, '{"mcpServers":{}}');
const beforeMcp = await readFile(mcpFile, 'utf8');
const server = createMockStore();
await new Promise(resolveListen => server.listen(0, '127.0.0.1', resolveListen));
const storeUrl = `http://127.0.0.1:${server.address().port}`;
const listener = createServer();
await new Promise(resolveListen => listener.listen(0, '127.0.0.1', resolveListen));
const port = listener.address().port;
await new Promise(resolveClose => listener.close(resolveClose));
const exe = resolve('src-tauri/target/debug/skills-manager.exe');
let child;
let browser;
const env = { ...process.env, COMPANY_STORE_MODE: 'mock', COMPANY_STORE_BASE_URL: storeUrl, COMPANY_PROFILE_ROOT: profile, COMPANY_DATA_ROOT: join(root, 'data'), WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}` };
async function startApp() {
  child = spawn(exe, [], { env, windowsHide: true, stdio: 'ignore' });
  let connected;
  for (let i = 0; i < 100; i++) {
    try { connected = await chromium.connectOverCDP(`http://127.0.0.1:${port}`); break; }
    catch { await new Promise(resolveWait => setTimeout(resolveWait, 200)); }
  }
  assert(connected, 'Native WebView2 did not expose the debug endpoint');
  browser = connected;
  let page;
  for (let i = 0; i < 50; i++) {
    page = browser.contexts().flatMap(context => context.pages()).find(candidate => !candidate.url().startsWith('devtools:'));
    if (page) break;
    await new Promise(resolveWait => setTimeout(resolveWait, 100));
  }
  assert(page, 'Native app page missing');
  page.setDefaultTimeout(15_000);
  await page.getByText('Dữ liệu giả lập', { exact: true }).waitFor();
  return page;
}
async function stopApp() {
  if (browser) { await browser.close().catch(() => {}); browser = undefined; }
  if (child && child.exitCode === null) {
    const exited = new Promise(resolveExit => child.once('exit', resolveExit));
    child.kill(); await exited;
  }
  child = undefined;
}
try {
  let page = await startApp();
  await page.getByLabel('Tài khoản', { exact: true }).fill('employee01');
  await page.getByLabel('Mật khẩu', { exact: true }).fill('demo-password');
  await page.getByRole('button', { name: 'Đăng nhập', exact: true }).click();
  await page.getByText('Nhân viên 01', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Cài đặt', exact: true }).click();
  await page.getByText('Phiên được giữ trong Windows Credential Manager.', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Store', exact: true }).click();
  await page.getByPlaceholder('Tìm theo tên hoặc công dụng skill…').fill('jira');
  await page.getByRole('button', { name: 'Xem chi tiết Tổng hợp tiến độ Jira', exact: true }).click();
  await page.getByRole('button', { name: 'Cài vào AIPRO Global', exact: true }).click();
  await page.getByText('Chưa có MCP trong cấu hình', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Dừng cài đặt', exact: true }).click();
  await page.getByRole('button', { name: 'Xem chi tiết Tổng hợp tiến độ Jira', exact: true }).click();
  await page.getByRole('button', { name: 'Cài vào AIPRO Global', exact: true }).click();
  await page.getByRole('button', { name: 'Bỏ qua và cài tiếp', exact: true }).click();
  await page.getByText('Cài xong; đã bỏ qua cảnh báo MCP của lần này.', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Hoàn tất', exact: true }).click();
  await page.getByPlaceholder('Tìm theo tên hoặc công dụng skill…').fill('excel');
  await page.getByRole('button', { name: 'Xem chi tiết Phân tích bảng tính Excel', exact: true }).click();
  await page.getByRole('button', { name: 'Cài vào AIPRO Global', exact: true }).click();
  await page.getByRole('button', { name: 'Bỏ qua và cài tiếp', exact: true }).click();
  await page.getByText('Cài xong; đã bỏ qua cảnh báo MCP của lần này.', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Hoàn tất', exact: true }).click();
  await page.getByRole('button', { name: 'Skill đã cài', exact: true }).click();
  await page.getByRole('heading', { name: 'Tổng hợp tiến độ Jira', exact: true }).waitFor();
  await page.getByRole('heading', { name: 'Phân tích bảng tính Excel', exact: true }).waitFor();
  assert.match(await readFile(join(profile, '.aipro/skills/excel-insights/SKILL.md'), 'utf8'), /Excel insights/);
  assert.equal(await readFile(mcpFile, 'utf8'), beforeMcp);
  const screenshot = process.env.COMPANY_SMOKE_SCREENSHOT;
  if (screenshot) { await mkdir(resolve(screenshot, '..'), { recursive: true }); await page.screenshot({ path: screenshot, fullPage: true }); }
  await stopApp();
  page = await startApp();
  await page.getByText('Nhân viên 01', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Skill đã cài', exact: true }).click();
  await page.getByRole('heading', { name: 'Phân tích bảng tính Excel', exact: true }).waitFor();
  await page.getByRole('button', { name: 'Đăng xuất', exact: true }).click();
  await page.getByText('Chưa đăng nhập', { exact: true }).waitFor();
  console.log('PASS native L1–L3: Windows Credential Manager, restart, Store search/detail, MCP stop/bypass, ZIP + 7z Global installation, unchanged MCP configuration, retained installations and logout.');
} finally {
  await stopApp(); await new Promise(resolveClose => server.close(resolveClose));
  const absolute = resolve(root);
  if (!absolute.startsWith(resolve(tmpdir()) + '\\') && !absolute.startsWith(resolve(tmpdir()) + '/')) throw new Error('Unsafe cleanup root');
  if (!basename(absolute).startsWith('company-native-l1-l3-')) throw new Error('Unexpected cleanup directory');
  await rm(absolute, { recursive: true, force: true });
}
