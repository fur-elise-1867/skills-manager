import { test } from 'node:test';
import assert from 'node:assert/strict';
import { makeLaunch } from './sds.mjs';

test('SDS build couples frontend, cargo feature and isolated capabilities', () => {
  const launch = makeLaunch('build', []);
  assert.equal(launch.env.VITE_APP_PROFILE, 'sds');
  assert.equal(launch.env.SDS_ENVIRONMENT, 'internal');
  assert.ok(launch.args.includes('sds'));
  assert.deepEqual(launch.config.app.security.capabilities, ['sds']);
  assert.equal(launch.config.identifier, 'com.sds.skillsmanager.desktop');
  assert.equal(launch.config.mainBinaryName, 'sds-skills-manager');
  assert.ok(launch.args.includes('--no-default-features'));
  assert.equal(launch.config.bundle.createUpdaterArtifacts, false);
  assert.equal(JSON.stringify(launch.config).includes('github.com'), false);
  assert.equal(launch.config.build.frontendDist, '../dist-sds');
});

test('local integration installer is development but never a fixture build', () => {
  const launch = makeLaunch('build', ['--environment', 'development', '--bundles', 'nsis']);
  assert.equal(launch.env.SDS_ENVIRONMENT, 'development');
  assert.equal(launch.env.SDS_FIXTURES, '0');
  assert.ok(launch.args.includes('nsis'));
  assert.equal(launch.args.includes('--environment'), false);
});

test('invalid environment, profile and feature overrides are rejected', () => {
  assert.throws(() => makeLaunch('build', ['--environment', 'production']), /environment/i);
  for (const override of ['--config', '--features', '--no-default-features']) {
    assert.throws(() => makeLaunch('build', [override, 'upstream']), /override/i);
  }
  for (const override of ['-c', '-f', '-c=other.json', '-fother', '--runner', '-r', '--']) {
    assert.throws(() => makeLaunch('build', [override, 'upstream']), /override/i);
  }
});

test('cargo check receives the same SDS configuration overlay as the build', () => {
  const launch = makeLaunch('check', []);
  const overlay = JSON.parse(launch.env.TAURI_CONFIG ?? '{}');
  assert.equal(overlay.identifier, 'com.sds.skillsmanager.desktop');
  assert.deepEqual(overlay.app.security.capabilities, ['sds']);
});

test('development launcher uses local assets and explicit fixture opt-in', () => {
  const launch = makeLaunch('dev', []);
  assert.equal(launch.env.SDS_ENVIRONMENT, 'development');
  assert.equal(launch.env.SDS_FIXTURES, '0');
  assert.match(launch.config.app.security.csp, /ws:\/\/127\.0\.0\.1/);
  assert.equal(launch.config.plugins.updater, undefined);
});

test('inherited fixture environment cannot split frontend and backend profiles', () => {
  const original = process.env.VITE_SDS_FIXTURES;
  try {
    process.env.VITE_SDS_FIXTURES = '1';
    assert.equal(makeLaunch('dev', []).env.VITE_SDS_FIXTURES, '0');
  } finally {
    if (original === undefined) delete process.env.VITE_SDS_FIXTURES;
    else process.env.VITE_SDS_FIXTURES = original;
  }
});
