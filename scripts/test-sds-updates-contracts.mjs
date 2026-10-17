import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const schema = JSON.parse(readFileSync(new URL('../contracts/sds-desktop/v1/types.json', import.meta.url)));
const fixture = JSON.parse(readFileSync(new URL('../contracts/sds-desktop/v1/fixtures/updates.json', import.meta.url)));
const ajv = new Ajv2020({strict: false, allErrors: true});
addFormats(ajv); ajv.addSchema(schema);
const validate = ajv.compile({$ref: `${schema.$id}#/definitions/updates`});
assert.ok(validate(fixture), ajv.errorsText(validate.errors));
for (const status of ['update_available', 'up_to_date', 'unavailable', 'unknown_release']) {
  const candidate = structuredClone(fixture);
  candidate.data.items[0].status = status;
  if (['unavailable', 'unknown_release'].includes(status)) candidate.data.items[0].latestRelease = null;
  assert.ok(validate(candidate), `${status}: ${ajv.errorsText(validate.errors)}`);
}
for (const mutate of [
  value => { delete value.data.checkedAt; },
  value => { value.data.checkedAt = 'yesterday'; },
  value => { delete value.data.items[0].currentReleaseId; },
  value => { value.data.items[0].status = 'downloaded'; },
  value => { value.data.items[0].status = 'unavailable'; },
  value => { value.data.items[0].latestRelease = null; },
]) {
  const candidate = structuredClone(fixture); mutate(candidate);
  assert.equal(validate(candidate), false, 'Unsupported or leaking update metadata must fail');
}
const validateRequest = ajv.compile({$ref: `${schema.$id}#/definitions/updates-request`});
for (const count of [1,100]) assert.ok(validateRequest({items:Array.from({length:count},(_,i)=>({skillId:`skill-${i}`,releaseId:'release-1'}))}));
for (const count of [0,101]) assert.equal(validateRequest({items:Array.from({length:count},(_,i)=>({skillId:`skill-${i}`,releaseId:'release-1'}))}),false);
assert.equal(validateRequest({items:[{skillId:'skill-1',releaseId:'release-1'},{skillId:'skill-1',releaseId:'release-1'}]}),false);
console.log('SDS update contracts passed: four states, metadata privacy, request bounds and duplicate rejection.');
