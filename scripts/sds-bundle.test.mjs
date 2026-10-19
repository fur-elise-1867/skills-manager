import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync, mkdtempSync, rmSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';

const root = process.cwd();
const temporaryBundleDir = process.env.SDS_BUNDLE_DIR ? null : mkdtempSync(path.join(os.tmpdir(), 'ax-portal-sds-'));
const bundleDir = process.env.SDS_BUNDLE_DIR ?? temporaryBundleDir;

if (temporaryBundleDir) {
  const vite = path.join(root, 'node_modules', 'vite', 'bin', 'vite.js');
  const build = spawnSync(process.execPath, [vite, 'build', '--mode', 'sds', '--outDir', temporaryBundleDir], {
    cwd: root,
    encoding: 'utf8',
  });
  assert.equal(build.status, 0, `SDS Vite build failed:\n${build.stderr}`);
}

process.on('exit', () => {
  if (temporaryBundleDir) rmSync(temporaryBundleDir, { recursive: true, force: true });
});

function containsCommand(bundle, command) {
  return ['"', "'", '`'].some((quote) => bundle.includes(`${quote}${command}${quote}`));
}

test('command boundary distinguishes SDS names from forbidden upstream commands', () => {
  assert.equal(containsCommand('invoke("sds_get_target_settings")', 'get_settings'), false);
  for (const quote of ['"', "'", '`']) {
    assert.equal(containsCommand(`invoke(${quote}get_settings${quote})`, 'get_settings'), true);
  }
});

test('SDS production bundle includes only its own entry and IPC boundary', () => {
  const assets = path.join(bundleDir, 'assets');
  const scripts = readdirSync(assets).filter((file) => file.endsWith('.js'));
  const bundle = scripts.map((file) => readFileSync(path.join(assets,file), 'utf8')).join('\n');
  assert.ok(bundle.includes('runtime_profile'), 'SDS runtime handshake missing from bundle');
  if(process.env.SDS_BUNDLE_DIR) {
    for(const command of ['sds_list_workspaces','sds_snapshots','sds_capture_snapshot','sds_prepare_restore','sds_check_updates','sds_prepare_updates','sds_execute_updates','sds_prepare_cache_delete','sds_prepare_cached_install']) {
      assert.ok(containsCommand(bundle,command), `Phase2 command missing from candidate bundle: ${command}`);
    }
  }
  for (const upstreamCommand of ['install_from_skillssh', 'git_backup_sync', 'get_settings', 'check_for_updates']) {
    assert.equal(containsCommand(bundle, upstreamCommand), false, `upstream command in SDS bundle: ${upstreamCommand}`);
  }
});

test('SDS production build output transforms HTML branding to AX Portal and favicon', () => {
  const htmlPath = path.join(bundleDir, 'index.html');
  const html = readFileSync(htmlPath, 'utf8');
  assert.ok(html.includes('<title>AX Portal</title>'), 'index.html should have title AX Portal');
  assert.ok(html.includes('/ax-portal/favicon.png'), 'index.html should have AX Portal favicon');
  assert.equal(html.includes('<title>Skills Manager</title>'), false, 'index.html must not contain legacy title');
  assert.equal(html.includes('/vite.svg'), false, 'index.html must not contain vite.svg favicon');
});

test('AX Portal branding assets and icon master exist in repository', () => {
  const requiredFiles = [
    'public/ax-portal/logo.png',
    'public/ax-portal/icon.png',
    'public/ax-portal/favicon.png',
    'src-tauri/icons/ax-portal/icon-source.png',
    'src-tauri/icons/ax-portal/icon.png',
    'src-tauri/icons/ax-portal/128x128@2x.png',
    'src-tauri/icons/ax-portal/128x128.png',
    'src-tauri/icons/ax-portal/64x64.png',
    'src-tauri/icons/ax-portal/32x32.png',
    'src-tauri/icons/ax-portal/icon.ico',
  ];
  for (const rel of requiredFiles) {
    assert.ok(existsSync(path.join(root, rel)), `Missing required asset: ${rel}`);
  }
});

test('SDS root fits within the native minimum-window client area', () => {
  const css = readFileSync(path.join(root, 'src', 'features', 'sds', 'sds.css'), 'utf8');
  assert.match(css, /\.sds-app\s*\{\s*min-width:\s*0;\s*min-height:\s*0;\s*\}/);
  assert.doesNotMatch(css, /\.sds-app\s*\{\s*min-width:\s*1100px/);
});

test('AX Portal handoff never records live credential values', () => {
  const handoff = readFileSync(path.join(root, 'docs', 'superpowers', 'evidence', '2026-10-19-ax-portal-gemini-handoff.md'), 'utf8');
  assert.doesNotMatch(handoff, /SDS_PROVIDER_(?:EMAIL|PASSWORD)\s*=\s*["'][^"']+["']/);
  assert.doesNotMatch(handoff, /admin@gmail\.com|Password/);
});

