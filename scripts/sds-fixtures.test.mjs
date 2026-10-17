import { test } from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import { startFixtureStore } from './sds-fixtures.mjs';

test('fixture update metadata is authenticated, bounded and never discloses unknown releases', async () => {
  const fixture = await startFixtureStore();
  try {
    const check = (items,authenticated=true) => fetch(`${fixture.baseUrl}/api/v1/desktop/updates/check`, {
      method:'POST',headers:{'content-type':'application/json',...(authenticated?{authorization:'Bearer synthetic-access-one'}:{})},body:JSON.stringify({items}),
    });
    const known = {skillId:'skill-123',releaseId:'rel-003'};
    assert.equal((await check([known],false)).status,401);
    const response = await check([known,{skillId:'skill-123',releaseId:'unknown'},{skillId:'hidden',releaseId:'unknown'}]);
    assert.equal(response.status,200);
    const body=await response.json();
    assert.deepEqual(body.data.items.map(item=>item.status),['up_to_date','unknown_release','unavailable']);
    assert.equal(body.data.items[0].latestRelease.sequence,3);
    assert.equal(body.data.items[1].latestRelease,null);
    assert.equal(body.data.items[2].latestRelease,null);
    assert.equal((await check([])).status,400);
    assert.equal((await check([known,known])).status,400);
    assert.equal((await check(Array.from({length:101},(_,i)=>({skillId:String(i),releaseId:'unknown'})))).status,400);
  } finally { await fixture.close(); }
});

test('explicit fixture HTTP Store accepts only its sample account and serves real desktop envelopes', async () => {
  const fixture = await startFixtureStore();
  try {
    assert.match(fixture.baseUrl,/^http:\/\/127\.0\.0\.1:\d+\/skm$/);
    const bad = await fetch(`${fixture.baseUrl}/api/v1/auth/login`, {method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({email:'someone@example.invalid',password:'synthetic-password'})});
    assert.equal(bad.status,400);
    const login = await fetch(`${fixture.baseUrl}/api/v1/auth/login`, {method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({email:'employee@example.invalid',password:'synthetic-password'})});
    const envelope = await login.json(); assert.equal(envelope.success,true); assert.equal(envelope.data.userId,1001);
    const headers = {authorization:`Bearer ${envelope.data.accessToken}`};
    const denied = await fetch(`${fixture.baseUrl}/api/v1/desktop/skills`); assert.equal(denied.status,401);
    const page = await (await fetch(`${fixture.baseUrl}/api/v1/desktop/skills?q=jira&mode=all_time&sourceId=quality&page=1&pageSize=20`,{headers})).json();
    assert.equal(page.data.totalItems,1); assert.equal(page.data.items[0].name,'Jira Weekly Report');
    const detail = await (await fetch(`${fixture.baseUrl}/api/v1/desktop/skills/skill-123`,{headers})).json();
    assert.equal(detail.data.latestRelease.skillId,'skill-123'); assert.deepEqual(detail.data.latestRelease.requiredMcps,['jira-mcp']);
    const metadata=await (await fetch(`${fixture.baseUrl}/api/v1/desktop/skills/skill-123/releases/rel-003`,{headers})).json();
    const download=await fetch(`${fixture.baseUrl}${metadata.data.package.downloadPath}`,{headers});
    assert.equal(download.status,200);
    const bytes=Buffer.from(await download.arrayBuffer());
    assert.equal(bytes.length,metadata.data.package.sizeBytes);
    assert.equal(createHash('sha256').update(bytes).digest('hex'),metadata.data.package.sha256);
    assert.equal((await fetch(`${fixture.baseUrl}${metadata.data.package.downloadPath}`)).status,401);
    assert.equal((await fetch(`${fixture.baseUrl}/api/v1/desktop/skills?mode=hot`,{headers})).status,422);
    assert.equal((await fetch(`${fixture.baseUrl}/api/v1/desktop/skills?q=__unavailable`,{headers})).status,503);
    const empty = await (await fetch(`${fixture.baseUrl}/api/v1/desktop/skills?q=__empty`,{headers})).json(); assert.equal(empty.data.totalItems,0);
  } finally { await fixture.close(); }
});
