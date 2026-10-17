import { readFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const readConfig = (name) => JSON.parse(readFileSync(path.join(root, 'src-tauri', name), 'utf8'));

// Match the JSON Merge Patch semantics used by Tauri's --config argument.
function merge(base, patch) {
  const result = { ...base };
  for (const [key, value] of Object.entries(patch)) {
    if (value === null) delete result[key];
    else if (typeof value === 'object' && !Array.isArray(value)) result[key] = merge(result[key] ?? {}, value);
    else result[key] = value;
  }
  return result;
}

export function makeLaunch(mode, rawArgs) {
  if (!['dev', 'build', 'check'].includes(mode)) throw new Error('Expected dev, build or check');
  const args = [...rawArgs];
  let environment = mode === 'dev' ? 'development' : 'internal';
  const option = args.indexOf('--environment');
  if (option >= 0) {
    environment = args[option + 1];
    args.splice(option, 2);
  }
  if (!['development', 'internal'].includes(environment)) throw new Error('Invalid SDS environment');
  const fixtures = args.includes('--fixtures');
  if (fixtures && (mode !== 'dev' || environment !== 'development')) throw new Error('Fixtures require local development mode and cannot be included in an installer');
  if (fixtures) args.splice(args.indexOf('--fixtures'),1);
  if (args.some((arg) => arg === '--' || /^-[cfr]/.test(arg) || /^(--config|--features|--no-default-features|--environment|--runner)(=|$)/.test(arg))) {
    throw new Error('SDS profile and feature overrides are not allowed');
  }
  let config = merge(readConfig('tauri.conf.json'), readConfig('tauri.sds.conf.json'));
  if (mode === 'dev') config = merge(config, readConfig('tauri.sds.dev.conf.json'));
  let overlay = readConfig('tauri.sds.conf.json');
  if (mode === 'dev') overlay = merge(overlay, readConfig('tauri.sds.dev.conf.json'));
  const env = {
    ...process.env,
    VITE_APP_PROFILE: 'sds',
    VITE_SDS_ENVIRONMENT: environment,
    SDS_ENVIRONMENT: environment,
    SDS_FIXTURES: fixtures ? '1' : '0',
    VITE_SDS_FIXTURES: fixtures ? '1' : '0',
  };
  delete env.TAURI_CONFIG;
  delete env.SDS_FIXTURE_URL;
  if (mode === 'check') env.TAURI_CONFIG = JSON.stringify(overlay);
  if (mode === 'check') return { command: 'cargo', args: ['check', '--manifest-path', 'src-tauri/Cargo.toml', '--no-default-features', '--features', 'sds'], env, config, fixtures };
  // Send the overlay, not the merged config: null removes upstream updater keys.
  return {
    command: process.execPath,
    args: [path.join(root, 'node_modules/@tauri-apps/cli/tauri.js'), mode, '--config', JSON.stringify(overlay), '--features', 'sds', ...args, '--', '--no-default-features'],
    env, config, fixtures,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    const launch = makeLaunch(process.argv[2], process.argv.slice(3));
    const fixture = launch.fixtures ? await (await import('./sds-fixtures.mjs')).startFixtureStore() : null;
    if (fixture) launch.env.SDS_FIXTURE_URL = fixture.baseUrl;
    const child = spawn(launch.command, launch.args, { cwd: root, env: launch.env, stdio: 'inherit', shell: false, windowsHide: true });
    child.on('error', (error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; void fixture?.close(); });
    child.on('exit', (code) => { process.exitCode = code ?? 1; void fixture?.close(); });
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
