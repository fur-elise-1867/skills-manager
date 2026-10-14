import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkReleaseInputs } from './check-company-release.mjs';

const valid = {
  tauri: { productName: 'Company Skills Manager', version: '1.42.0', identifier: 'com.companyskills.manager', plugins: {}, bundle: { targets: ['nsis'], createUpdaterArtifacts: false, windows: { webviewInstallMode: { type: 'skip' } } } },
  store: { baseUrl: '', allowedHosts: [] }, permissions: ['core:default', 'dialog:allow-open'],
  entry: 'company_app::run()', frontend: '<CompanyApp />', runtime: 'company_environment', workflow: 'workflow_dispatch: {}\ncontents: read',
};
test('release gate allows an explicitly unconfigured Store and private Windows artifacts', () => {
  assert.deepEqual(checkReleaseInputs(valid), []);
});
test('release gate rejects sample identity, updater, public publishing, broad file permissions and mock transport', () => {
  for (const mutate of [
    v => { v.tauri.identifier = 'com.agentskills.desktop'; },
    v => { v.tauri.plugins.updater = { endpoints: ['https://github.com/public/latest'] }; },
    v => { v.tauri.bundle.createUpdaterArtifacts = true; },
    v => { v.workflow += '\ngh release create example'; },
    v => { v.permissions.push('fs:default'); },
    v => { v.store.baseUrl = 'http://127.0.0.1:4318'; v.store.allowedHosts = ['127.0.0.1']; },
    v => { v.store.baseUrl = 'https://skills-store.company.example'; v.store.allowedHosts = ['skills-store.company.example']; },
  ]) { const input = structuredClone(valid); mutate(input); assert(checkReleaseInputs(input).length > 0); }
});
