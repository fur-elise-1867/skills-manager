import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('generated SDS installer targets only its own process and excludes the upstream CLI', () => {
  const profile = process.env.SDS_INSTALLER_PROFILE ?? 'debug';
  assert.ok(['debug','release'].includes(profile));
  const script = readFileSync(`src-tauri/target/${profile}/nsis/x64/installer.nsi`, 'utf8');
  assert.ok(script.includes('!define MAINBINARYNAME "sds-skills-manager"'), 'installer must use a separate SDS process name');
  assert.ok(script.includes('!define BUNDLEID "com.sds.skillsmanager.desktop"'));
  assert.equal(script.includes('skills-manager-cli.exe'), false, 'installer must not bundle the CLI');
  assert.equal(script.includes('github.com/xingkongliang'), false);
});
