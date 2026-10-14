import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function checkReleaseInputs({ tauri, store, permissions, entry, frontend, runtime, workflow }) {
  const errors = [];
  if (tauri.identifier !== 'com.companyskills.manager' || tauri.productName !== 'Company Skills Manager') errors.push('Company identity is required');
  if (tauri.plugins?.updater || tauri.bundle?.createUpdaterArtifacts !== false) errors.push('Public updater must be disabled');
  if (JSON.stringify(tauri.bundle?.targets) !== '["nsis"]') errors.push('Delivery targets must be Windows NSIS only');
  if (tauri.bundle?.windows?.webviewInstallMode?.type !== 'skip') errors.push('Install must use preinstalled/offline WebView2 without a network bootstrapper');
  if (permissions.some(p => /^(fs|shell|updater):/.test(p))) errors.push('Frontend must not have broad filesystem/shell/updater access');
  if (!entry.includes('company_app::run()') || !frontend.includes('CompanyApp') || !runtime.includes('company_environment')) errors.push('Company entry is required');
  if (/gh\s+release|VERCEL_DEPLOY_HOOK|refs\/tags|tags:/.test(workflow)) errors.push('Company build must not auto-publish a public release');
  if (!Array.isArray(store.allowedHosts)) errors.push('Store allowlist is required');
  if (store.baseUrl) {
    try {
      const url = new URL(store.baseUrl);
      if (url.protocol !== 'https:' || ['localhost', '127.0.0.1', '[::1]', 'example.com', 'example.org', 'example.net'].includes(url.hostname) || /\.(example|invalid)$/.test(url.hostname) || url.username || url.password || url.search || url.hash || url.pathname !== '/' || !store.allowedHosts?.some(h => h.trim().toLowerCase() === url.hostname)) errors.push('Store must be an explicitly allowed HTTPS origin');
    } catch { errors.push('Invalid Store URL'); }
  }
  return errors;
}

export async function checkCompanyRelease(root = resolve('.')) {
  const read = path => readFile(resolve(root, path), 'utf8');
  const [tauri, store, capabilities, entry, frontend, runtime, workflow] = await Promise.all([
    read('src-tauri/tauri.conf.json'), read('src-tauri/resources/company-store.json'), read('src-tauri/capabilities/default.json'),
    read('src-tauri/src/main.rs'), read('src/App.tsx'), read('src-tauri/src/company_app.rs'), read('.github/workflows/release.yml'),
  ]);
  const config = JSON.parse(tauri);
  const errors = checkReleaseInputs({ tauri: config, store: JSON.parse(store), permissions: JSON.parse(capabilities).permissions, entry, frontend, runtime, workflow });
  const [packageText, cargoText] = await Promise.all([read('package.json'), read('src-tauri/Cargo.toml')]);
  const cargoVersion = cargoText.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  if (JSON.parse(packageText).version !== config.version || cargoVersion !== config.version) errors.push('Package/Tauri/Cargo versions must match');
  return errors;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const errors = await checkCompanyRelease();
  if (errors.length) { for (const error of errors) console.error(error); process.exitCode = 1; }
  else console.log('PASS company Windows release gate (configuration only; actual Store/AIPRO acceptance remains separate)');
}
