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
let child; let browser; let page; let storeClosed = false;
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
  if (process.argv.includes('--lifecycle')) {
    await new Promise(done => server.close(done)); storeClosed = true;
    const aCard = () => page.locator('.installation-card').filter({ hasText: 'Dự án: Project A' });
    await aCard().getByRole('button', { name: 'Bản lưu', exact: true }).click();
    await page.getByRole('button', { name: 'Khôi phục bản lưu', exact: true }).click();
    assert.match(await readFile(join(aTarget, 'SKILL.md'), 'utf8'), /Jira project summary v2/);
    await page.getByRole('button', { name: 'Backup và khôi phục', exact: true }).click();
    await page.getByText('Đã khôi phục nội dung và baseline của bản lưu.', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Đóng', exact: true }).click();
    assert.equal(await readFile(join(aTarget, 'SKILL.md'), 'utf8'), 'Native project customization');
    const restored = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('company_installations_list', {}));
    assert.equal(restored.find(i => i.id === backup.installationId).packageHash, backup.installation.packageHash);
    await aCard().getByRole('button', { name: 'Gỡ bản cài', exact: true }).click();
    await page.getByRole('button', { name: 'Backup và gỡ', exact: true }).click();
    await page.getByText('Đã gỡ bản cài; bản lưu vẫn có thể khôi phục.', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Đóng', exact: true }).click();
    await assert.rejects(readFile(join(aTarget, 'SKILL.md')));
    assert.deepEqual(await readFile(join(globalTarget, 'SKILL.md')), originalGlobal);
    assert.deepEqual(await readFile(join(bTarget, 'SKILL.md')), originalB);
    await stop(); await start();
    await page.getByRole('button', { name: 'Skill đã cài', exact: true }).click();
    await aCard().getByText('Đã gỡ', { exact: true }).waitFor();
    await aCard().getByRole('button', { name: 'Bản lưu', exact: true }).click();
    const removedBackup = page.locator('.company-card article').filter({ hasText: 'Trước khi gỡ' });
    await removedBackup.getByRole('button', { name: 'Khôi phục bản lưu', exact: true }).click();
    await page.getByRole('button', { name: 'Backup và khôi phục', exact: true }).click();
    await page.getByText('Đã khôi phục nội dung và baseline của bản lưu.', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Đóng', exact: true }).click();
    assert.equal(await readFile(join(aTarget, 'SKILL.md'), 'utf8'), 'Native project customization');
    assert.equal(await readFile(mcp, 'utf8'), mcpBefore);
    if (screenshot) await page.screenshot({ path: screenshot, fullPage: true });
    console.log('PASS native L6: offline restore/baseline, backup/remove, restart/restore, independent targets and unchanged MCP');
  }
  if (process.argv.includes('--local')) {
    assert(storeClosed, '--local runs after --lifecycle with the Store offline');
    for (const [file, slug] of [['writing-review.zip', 'local-writing'], ['excel-insights.7z', 'excel-insights']]) {
      await page.getByRole('button', { name: 'Nhập ZIP/7z', exact: true }).click();
      await page.getByLabel('File ZIP/7z', { exact: true }).fill(resolve('fixtures/company-store/packages', file));
      await page.getByLabel('Tên thư mục skill', { exact: true }).fill(slug);
      await page.getByRole('button', { name: 'Kiểm tra gói local', exact: true }).click();
      await page.getByText('Gói local chưa có dữ liệu yêu cầu MCP từ Skill Store. Không suy ra yêu cầu từ SKILL.md.', { exact: true }).waitFor();
      await page.getByRole('button', { name: 'Bỏ qua và cài tiếp', exact: true }).click();
      await page.getByText('Cài xong; đã bỏ qua cảnh báo MCP của lần này.', { exact: true }).waitFor();
      await page.getByRole('button', { name: 'Hoàn tất', exact: true }).click();
      const local = await page.evaluate(async slug => (await window.__TAURI_INTERNALS__.invoke('company_installations_list', {})).find(i => i.slug === slug), slug);
      assert.equal(local.origin, 'local'); assert.equal(local.skillId, null); assert.equal(local.storeState, 'not_applicable');
      assert(await readFile(join(local.target, 'SKILL.md'), 'utf8'));
    }
    const localCard = page.locator('.installation-card').filter({ has: page.getByRole('heading', { name: 'local-writing', exact: true }) });
    await localCard.getByRole('button', { name: 'Gỡ bản cài', exact: true }).click();
    await page.getByRole('button', { name: 'Backup và gỡ', exact: true }).click();
    await page.getByText('Đã gỡ bản cài; bản lưu vẫn có thể khôi phục.', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Đóng', exact: true }).click();
    await page.getByRole('button', { name: 'Cài đặt', exact: true }).click();
    const storage = await page.evaluate(async () => window.__TAURI_INTERNALS__.invoke('company_storage_status', {}));
    const localBackup = storage.backups.find(b => b.backup.installation.slug === 'local-writing'); assert(localBackup);
    const storageBackupCard = page.locator('.company-card article').filter({ hasText: 'local-writing' });
    await storageBackupCard.getByRole('button', { name: 'Xóa bản lưu', exact: true }).click();
    await page.getByRole('button', { name: 'Hủy', exact: true }).click();
    assert(await readFile(join(localBackup.backup.path, 'SKILL.md'), 'utf8'));
    await storageBackupCard.getByRole('button', { name: 'Xóa bản lưu', exact: true }).click();
    await page.getByRole('button', { name: 'Xác nhận xóa', exact: true }).click();
    await page.getByLabel(`Chọn gói ${localBackup.backup.installation.packageHash}`, { exact: true }).waitFor();
    const unused = page.getByLabel(`Chọn gói ${localBackup.backup.installation.packageHash}`, { exact: true });
    await unused.check();
    await page.getByRole('button', { name: 'Dọn các gói đã chọn (1)', exact: true }).click();
    await page.getByRole('button', { name: 'Xác nhận xóa', exact: true }).click();
    await unused.waitFor({ state: 'detached' });
    await assert.rejects(readFile(join(localBackup.backup.path, 'SKILL.md')));
    assert.equal(await readFile(mcp, 'utf8'), mcpBefore);
    if (screenshot) await page.screenshot({ path: screenshot, fullPage: true });
    console.log('PASS native L7: offline local ZIP/7z, local origin/no Store id, retained backup, cancel/delete/unused-package cleanup');
  }
  console.log('PASS native L4/L5: projects, independent copies, customization, update preview/backup/apply, restart and unchanged MCP');
} finally {
  if (page) await page.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke('company_logout', {}); } catch { /* owned mock credentials only */ } }).catch(() => {});
  await stop(); if (!storeClosed) await new Promise(done => server.close(done));
  assert.equal(dirname(resolve(root)), resolve(tmpdir())); assert(basename(root).startsWith('company-native-l4-l5-'));
  await rm(root, { recursive: true, force: true });
}
