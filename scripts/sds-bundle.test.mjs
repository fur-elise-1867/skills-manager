import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';

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
  const assets = path.join(process.env.SDS_BUNDLE_DIR ?? 'dist-sds','assets');
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
