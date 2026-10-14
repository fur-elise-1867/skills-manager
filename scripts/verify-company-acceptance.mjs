import { spawn } from 'node:child_process';
import { access, mkdtemp, readFile, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join, resolve, dirname, basename, sep } from 'node:path';
import assert from 'node:assert/strict';

const version = JSON.parse(await readFile('package.json', 'utf8')).version;
const pack = resolve(process.argv[2] || `docs/delivery/2026-10-14-aipro-l6-l8-acceptance/Company-Skills-Manager-${version}-Windows-Acceptance`);
const manifest = JSON.parse(await readFile(join(pack, 'MANIFEST.json'), 'utf8'));
assert.equal(manifest.version, version); assert.equal(manifest.realStoreVerified, false); assert.equal(manifest.realAiproVerified, false);
for (const file of manifest.files) {
  const path = resolve(pack, file.path); assert(path.startsWith(pack + sep));
  const data = await readFile(path); assert.equal(data.length, file.bytes, file.path);
  assert.equal(createHash('sha256').update(data).digest('hex'), file.sha256, file.path);
}
for (const path of ['app/skills-manager.exe', 'runtime/node.exe', 'prerequisites/MicrosoftEdgeWebView2RuntimeInstallerX64.exe', 'prerequisites/VC_redist.x64.exe', '01-Run-Mock-Demo.cmd', 'ACCEPTANCE.md']) await access(join(pack, path));
const root = await mkdtemp(join(tmpdir(), 'company-acceptance-check-'));
const child = spawn(join(pack, 'runtime/node.exe'), [join(pack, 'mock/run-demo.mjs'), '--server-only', '--port', '0', '--data-root', root], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
let output = ''; let errors = ''; child.stdout.on('data', chunk => output += chunk); child.stderr.on('data', chunk => errors += chunk);
try {
  const end = Date.now() + 15000;
  while (!output.includes('SERVER_READY')) { assert(child.exitCode === null, errors); assert(Date.now() < end, 'Bundled launcher startup timeout'); await new Promise(done => setTimeout(done, 50)); }
  const url = output.match(/SERVER_READY (http:\/\/127\.0\.0\.1:\d+)/)[1];
  const login = await fetch(`${url}/api/v1/auth/login`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ username: 'employee01', password: 'demo-password' }) });
  assert.equal(login.status, 200);
  const token = (await login.json()).data.access_token;
  const headers = { Authorization: `Bearer ${token}` };
  const detail = (await (await fetch(`${url}/api/v1/skills/skl_002`, { headers })).json()).data;
  const response = await fetch(`${url}/api/v1/skills/skl_002/package`, { headers: { ...headers, 'If-Match': detail.package.etag } });
  assert.equal(response.status, 200);
  const bytes = Buffer.from(await response.arrayBuffer()); assert.equal(createHash('sha256').update(bytes).digest('hex'), detail.package.sha256);
  assert.equal(bytes.length, detail.package.size_bytes);
  await fetch(`${url}/api/v1/auth/logout`, { method: 'POST', headers });
  console.log(`PASS acceptance kit: ${manifest.files.length} file hashes, bundled Node/launcher, auth/detail/7z/ETag/hash and isolated demo roots`);
} finally {
  if (child.exitCode === null) { const exit = new Promise(done => child.once('exit', done)); child.kill(); await exit; }
  assert.equal(dirname(resolve(root)), resolve(tmpdir())); assert(basename(root).startsWith('company-acceptance-check-'));
  await rm(root, { recursive: true, force: true });
}
