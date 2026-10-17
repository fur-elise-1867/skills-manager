// Opt-in contract check against the isolated local Store. Tokens stay in memory.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const base = new URL(process.env.SDS_PROVIDER_URL ?? 'http://127.0.0.1:8080/skm/');
assert.equal(base.protocol, 'http:');
assert.equal(base.hostname, '127.0.0.1', 'Only the dedicated loopback Store is supported');
assert.equal(base.pathname, '/skm/');
assert.ok(!base.username && !base.password && !base.search && !base.hash);
const email = process.env.SDS_PROVIDER_EMAIL;
const password = process.env.SDS_PROVIDER_PASSWORD;
assert.ok(email && password, 'Set SDS_PROVIDER_EMAIL and SDS_PROVIDER_PASSWORD in the process environment');
const schema = JSON.parse(readFileSync(new URL('../contracts/sds-desktop/v1/types.json', import.meta.url)));
const ajv = new Ajv2020({strict:false, allErrors:true});
addFormats(ajv); ajv.addSchema(schema);
const validate = ajv.compile({$ref:`${schema.$id}#/definitions/updates`});

async function request(route, {token,body,status=200}={}) {
  const response = await fetch(new URL(route,base), {
    method:body ? 'POST' : 'GET', redirect:'error', signal:AbortSignal.timeout(15000),
    headers:{...(token ? {Authorization:`Bearer ${token}`} : {}),...(body ? {'Content-Type':'application/json'} : {})},
    body:body ? JSON.stringify(body) : undefined,
  });
  assert.equal(response.status,status,`${route}: unexpected HTTP status`);
  return response;
}
const login = await (await request('api/v1/auth/login',{body:{email,password}})).json();
assert.equal(login.success,true);
assert.ok(login.data?.accessToken);
const token=login.data.accessToken;
try {
  const catalog=await (await request('api/v1/desktop/skills?page=1&pageSize=100',{token})).json();
  const known = process.env.SDS_UPDATES_ITEMS
    ? JSON.parse(process.env.SDS_UPDATES_ITEMS)
    : catalog.data.items.filter(item=>item.latestRelease && item.canDownload)
      .map(item=>({skillId:item.id,releaseId:item.latestRelease.id}));
  assert.ok(known.length>0 && known.length<=100,'Need 1–100 authorized known releases for live checks');
  const route='api/v1/desktop/updates/check';
  const check=await (await request(route,{token,body:{items:known}})).json();
  assert.ok(validate(check),ajv.errorsText(validate.errors));
  assert.equal(check.data.items.length,known.length);
  assert.deepEqual(new Set(check.data.items.map(item=>JSON.stringify([item.skillId,item.currentReleaseId]))),
    new Set(known.map(item=>JSON.stringify([item.skillId,item.releaseId]))));
  for (const item of check.data.items) assert.ok(['up_to_date','update_available'].includes(item.status),'Known authorized releases must have an explicit result');
  await request(route,{body:{items:known},status:401});
  await request(route,{token,body:{items:[]},status:400});
  await request(route,{token,body:{items:[known[0],known[0]]},status:400});
  const boundary = Array.from({length:100},(_,index)=>({skillId:String(900000000+index),releaseId:'00000000-0000-0000-0000-000000000000'}));
  const hundred=await (await request(route,{token,body:{items:boundary}})).json();
  assert.ok(validate(hundred),ajv.errorsText(validate.errors));
  assert.equal(hundred.data.items.length,100);
  for (const item of hundred.data.items) {
    assert.ok(['unavailable','unknown_release'].includes(item.status));
    assert.ok(item.latestRelease==null,'Unknown or denied content must not disclose latest release');
  }
  await request(route,{token,body:{items:[...boundary,{skillId:'900000101',releaseId:boundary[0].releaseId}]},status:400});
  console.log(`Live updates provider passed: ${known.length} authorized releases, 100-result boundary, 0/101/duplicate rejection, auth and denied metadata privacy; no package downloads.`);
} finally {
  await request('api/v1/auth/logout',{token,body:{refreshToken:login.data.refreshToken}});
}
