// Opt-in HTTP contract check against the dedicated running Store pilot.
// Credentials stay in environment/memory; responses containing tokens are never logged.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const base = new URL(process.env.SDS_PROVIDER_URL ?? 'http://127.0.0.1:8080/skm/');
assert.equal(base.protocol, 'http:');
assert.equal(base.hostname, '127.0.0.1', 'This harness only targets the local pilot');
assert.equal(base.pathname, '/skm/');
const email = process.env.SDS_PROVIDER_EMAIL;
const password = process.env.SDS_PROVIDER_PASSWORD;
assert.ok(email && password, 'Set SDS_PROVIDER_EMAIL and SDS_PROVIDER_PASSWORD');
const schema = JSON.parse(readFileSync(new URL('../contracts/sds-desktop/v1/types.json', import.meta.url)));
const ajv = new Ajv2020({strict: false, allErrors: true});
addFormats(ajv); ajv.addSchema(schema);
function validate(name, value) {
  const check = ajv.compile({$ref: `${schema.$id}#/definitions/${name}`});
  assert.ok(check(value), `${name}: ${ajv.errorsText(check.errors)}`);
}
async function request(route, {token, body, status = 200} = {}) {
  const response = await fetch(new URL(route, base), {
    method: body ? 'POST' : 'GET', redirect: 'error', signal: AbortSignal.timeout(15000),
    headers: {...(token ? {Authorization: `Bearer ${token}`} : {}), ...(body ? {'Content-Type': 'application/json'} : {})},
    body: body ? JSON.stringify(body) : undefined,
  });
  assert.equal(response.status, status, `${route}: unexpected status`);
  return response;
}
const login = await (await request('api/v1/auth/login', {body: {email, password}})).json();
validate('auth-login', login);
const refreshed = await (await request('api/v1/auth/refresh', {body: {refreshToken: login.data.refreshToken}})).json();
validate('auth-refresh', refreshed);
const token = refreshed.data.accessToken;
const me = await (await request('api/v1/desktop/me', {token})).json();
assert.equal(me.success, true); assert.equal(me.data.id, String(login.data.userId));
const taxonomy = await (await request('api/v1/desktop/taxonomy', {token})).json();
validate('taxonomy', taxonomy);
const catalog = await (await request('api/v1/desktop/skills?q=SDS%20Pilot&sourceId=desktop-pilot&page=1&pageSize=100', {token})).json();
validate('catalog', catalog);
assert.ok(catalog.data.items.length >= 2);
const ordered = catalog.data.items.map(s => s.name.toLowerCase());
assert.deepEqual(ordered, [...ordered].sort());
for (const [skillId, id, format, expectedMcps] of [
  ['1', '8b523c68-f7e5-4ea6-a1a2-cdfd4593c105', 'zip', ['jira-mcp']],
  ['2', '94de5a6f-5e44-4770-9e86-6a8da12d7fe5', '7z', []],
]) {
  const route = `api/v1/desktop/skills/${skillId}/releases/${id}`;
  const detail = await (await request(`api/v1/desktop/skills/${skillId}`, {token})).json();
  validate('detail', detail);
  const release = await (await request(route, {token})).json();
  validate('release', release);
  assert.deepEqual(release.data.requiredMcps, expectedMcps);
  assert.ok(detail.data.latestRelease.sequence >= release.data.sequence);
  if (detail.data.latestRelease.sequence === release.data.sequence) assert.equal(detail.data.latestRelease.id, id);
  if (skillId === '2' && detail.data.latestRelease.sequence === 2) assert.equal(detail.data.latestRelease.requiredMcps, null);
  assert.equal(release.data.package.downloadPath, `/${route}/package`);
  await request(route, {status: 401});
  await request(`${route}/package`, {status: 401});
  const response = await request(`${route}/package`, {token});
  const bytes = Buffer.from(await response.arrayBuffer());
  const expected = readFileSync(new URL(`../contracts/sds-desktop/v1/fixtures/packages/sds-synthetic.${format}`, import.meta.url));
  assert.deepEqual(bytes, expected, 'HTTP must serve original archive bytes');
  assert.equal(bytes.length, release.data.package.sizeBytes);
  assert.equal(createHash('sha256').update(bytes).digest('hex'), release.data.package.sha256);
  assert.equal(Number(response.headers.get('content-length')), bytes.length);
  assert.equal(response.headers.get('content-type'), format === 'zip' ? 'application/zip' : 'application/x-7z-compressed');
  assert.match(response.headers.get('content-disposition'), /attachment/);
  assert.match(response.headers.get('cache-control'), /no-store/);
  console.log(`Verified authenticated ${format}: ${bytes.length} exact bytes, metadata and checksum`);
}
for (const mode of ['trending', 'hot']) {
  const body = await (await request(`api/v1/desktop/skills?mode=${mode}`, {token, status: 422})).json();
  assert.equal(body.success, false);
}
await request('api/v1/desktop/skills?page=0', {token, status: 400});
await request('api/v1/desktop/skills/1/releases/not-a-uuid', {token, status: 404});
await request('api/v1/desktop/skills/1/releases/00000000-0000-0000-0000-000000000000/package', {token, status: 404});
await request('api/v1/auth/logout', {token, body: {refreshToken: refreshed.data.refreshToken}});
console.log('Live Store provider contracts, refresh, catalog/detail, failures and package streams passed');
