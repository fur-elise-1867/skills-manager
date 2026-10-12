import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createMockStore } from './company-mock-store.mjs';

test('Store session permits catalog access and logout revokes only that token', async t => {
  const server = createMockStore();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => server.close());
  const root = `http://127.0.0.1:${server.address().port}/api/v1`;
  assert.equal((await fetch(`${root}/skills`)).status, 401);
  const response = await fetch(`${root}/auth/login`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ username: 'employee01', password: 'demo-password' }) });
  assert.equal(response.status, 200);
  const { data } = await response.json();
  const headers = { Authorization: `Bearer ${data.access_token}` };
  assert.equal((await fetch(`${root}/skills`, { headers })).status, 200);
  assert.equal((await fetch(`${root}/auth/logout`, { method: 'POST', headers })).status, 204);
  assert.equal((await fetch(`${root}/skills`, { headers })).status, 401);
  assert.equal((await fetch(`${root}/auth/logout`, { method: 'POST', headers })).status, 204);
});

test('detail preserves MCP description and archive download matches selected metadata', async t => {
  const server = createMockStore();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => server.close());
  const root = `http://127.0.0.1:${server.address().port}/api/v1`;
  const login = await fetch(`${root}/auth/login`, { method: 'POST', body: JSON.stringify({ username: 'employee01', password: 'demo-password' }) });
  const { data: session } = await login.json();
  const headers = { Authorization: `Bearer ${session.access_token}` };
  const { data: list } = await (await fetch(`${root}/skills?q=jira`, { headers })).json();
  assert.equal(list.items.length, 1);
  const { data: detail } = await (await fetch(`${root}/skills/skl_001`, { headers })).json();
  assert.match(detail.description, /\n\nMCP bắt buộc: jira-mcp, confluence-mcp\n/);
  assert.equal((await fetch(`${root}/skills/skl_001/package`, { headers: { ...headers, 'If-Match': '"wrong"' } })).status, 412);
  const archive = await fetch(`${root}/skills/skl_001/package`, { headers: { ...headers, 'If-Match': detail.package.etag } });
  assert.equal(archive.status, 200);
  assert.equal((await archive.arrayBuffer()).byteLength, detail.package.size_bytes);
});
