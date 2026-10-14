import { cp, copyFile, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync } from 'node:fs';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const version = JSON.parse(await readFile(join(repo, 'package.json'), 'utf8')).version;
const delivery = join(repo, 'docs/delivery/2026-10-14-aipro-l6-l8-acceptance');
const pack = join(delivery, `Company-Skills-Manager-${version}-Windows-Acceptance`);
const previous = join(repo, 'docs/delivery/2026-10-14-win11-customer-trial/Company-Skills-Manager-1.41.0-Win11-x64-Trial');
const nodeRuntime = process.env.COMPANY_ACCEPTANCE_NODE_DIR || join(previous, 'runtime');
const prerequisites = process.env.COMPANY_ACCEPTANCE_PREREQUISITES_DIR || join(previous, 'prerequisites');
if (!existsSync(join(nodeRuntime, 'node.exe')) || !existsSync(join(nodeRuntime, 'LICENSE')) || !existsSync(join(prerequisites, 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe')) || !existsSync(join(prerequisites, 'VC_redist.x64.exe'))) {
  throw new Error('Provide offline Node+LICENSE and Windows runtime installers through COMPANY_ACCEPTANCE_NODE_DIR / COMPANY_ACCEPTANCE_PREREQUISITES_DIR (the existing delivery cache is the default).');
}
const inputs = [
  ['src-tauri/target/debug/skills-manager.exe', 'app/skills-manager.exe'],
  ['scripts/company-acceptance-launcher.mjs', 'mock/run-demo.mjs'],
  ['scripts/company-mock-store.mjs', 'mock/company-mock-store.mjs'],
  ['src-tauri/resources/company-store.json', 'config/company-store.json'],
  ['src-tauri/resources/company-store.json', 'app/resources/company-store.json'],
  ['docs/validation/company-aipro-acceptance.md', 'ACCEPTANCE.md'],
  ['COMPANY_APP_README.md', 'DEVELOPER-README.md'],
  ['LICENSE', 'licenses/APP-LICENSE'],
];
for (const [source, destination] of inputs) {
  await mkdir(dirname(join(pack, destination)), { recursive: true });
  await copyFile(join(repo, source), join(pack, destination));
}
await cp(join(repo, 'fixtures/company-store'), join(pack, 'fixtures/company-store'), { recursive: true });
await cp(join(repo, 'licenses'), join(pack, 'licenses'), { recursive: true });
await cp(join(repo, 'docs/api/skills-store-l1-l5'), join(pack, 'documents/skills-store-api'), { recursive: true });
await cp(nodeRuntime, join(pack, 'runtime'), { recursive: true });
await cp(prerequisites, join(pack, 'prerequisites'), { recursive: true });
for (const mode of ['debug', 'release']) {
  const file = join(repo, `src-tauri/target/${mode}/bundle/nsis/Company Skills Manager_${version}_x64-setup.exe`);
  if (!existsSync(file)) throw new Error(`Build ${mode} installer ${version} before packaging`);
  await mkdir(join(pack, 'installers', mode), { recursive: true });
  await copyFile(file, join(pack, 'installers', mode, basename(file)));
}
await writeFile(join(pack, '01-Run-Mock-Demo.cmd'), '@echo off\r\nchcp 65001 >nul\r\n"%~dp0runtime\\node.exe" "%~dp0mock\\run-demo.mjs"\r\nif errorlevel 1 pause\r\n');
await writeFile(join(pack, '02-Publish-Mock-Jira-v2.cmd'), '@echo off\r\nchcp 65001 >nul\r\n"%~dp0runtime\\node.exe" "%~dp0mock\\run-demo.mjs" --publish-update\r\npause\r\n');
await writeFile(join(pack, 'START-HERE.txt'), `Company Skills Manager ${version} — AIPRO acceptance\r\n\r\n1. Extract/copy the entire folder to Windows x64.\r\n2. If needed, have IT install the signed offline WebView2 and VC runtimes in prerequisites.\r\n3. Run 01-Run-Mock-Demo.cmd; keep its terminal open. No Node/7-Zip/Python installation is required.\r\n4. Mock account: employee01 or employee02 / demo-password.\r\n5. Demo data: %LOCALAPPDATA%\\CompanySkillsManagerAiproAcceptance142 (isolated from real AIPRO).\r\n6. Use 02-Publish-Mock-Jira-v2.cmd to demonstrate updates.\r\n7. Follow ACCEPTANCE.md for L1–L7, then record Store/AIPRO/IT integration separately when available.\r\n\r\nThe portable demo uses the debug build to permit isolated loopback mock. Release installer rejects mock/debug overrides and has an empty Store configuration until IT configures allowed HTTPS. Installers are not Authenticode signed by IT. They require preinstalled/offline WebView2 and do not download it.\r\n`);
const manifest = { version, kind: 'mock-acceptance', realStoreVerified: false, realAiproVerified: false, files: [] };
async function files(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (path === join(pack, 'MANIFEST.json') || path === join(pack, 'SHA256SUMS.txt')) continue;
    if (entry.isDirectory()) await files(path);
    else if (entry.isFile()) {
      const data = await readFile(path);
      manifest.files.push({ path: path.slice(pack.length + 1).replaceAll('\\', '/'), bytes: data.length, sha256: createHash('sha256').update(data).digest('hex') });
    } else throw new Error('Acceptance kit must not contain links');
  }
}
await files(pack);
manifest.files.sort((a, b) => a.path.localeCompare(b.path));
await writeFile(join(pack, 'MANIFEST.json'), JSON.stringify(manifest, null, 2) + '\n');
await writeFile(join(pack, 'SHA256SUMS.txt'), manifest.files.map(f => `${f.sha256}  ${f.path}`).join('\n') + '\n');
console.log(`Packaged mock acceptance kit: ${pack}\n${manifest.files.length} files with SHA-256 manifest`);
