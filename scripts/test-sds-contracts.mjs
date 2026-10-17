import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';
import ts from 'typescript';

const root=fileURLToPath(new URL('../',import.meta.url));
const contract=path.join(root,'contracts/sds-desktop/v1');
const schema=JSON.parse(readFileSync(path.join(contract,'types.json'),'utf8'));
const ajv=new Ajv2020({strict:false,allErrors:true});
addFormats(ajv);
ajv.addSchema(schema);
const fixtures={};
for(const name of ['auth-login','auth-refresh','catalog','detail','release','taxonomy']) {
  const fixture=JSON.parse(readFileSync(path.join(contract,'fixtures',`${name}.json`),'utf8'));
  const validate=ajv.compile({$ref:`${schema.$id}#/definitions/${name}`});
  assert.ok(validate(fixture),`${name}: ${ajv.errorsText(validate.errors)}`);
  const extended=structuredClone(fixture); extended.futureField={enabled:true};
  assert.ok(validate(extended),`${name}: unknown fields must remain compatible`);
  const missing=structuredClone(fixture); delete missing.data;
  assert.equal(validate(missing),false,`${name}: missing data must fail`);
  fixtures[name]=fixture;
}
for(const name of ['catalog','detail']) {
  const validate=ajv.compile({$ref:`${schema.$id}#/definitions/${name}`});
  for(const key of schema.definitions['skill-metadata'].required) {
    const missing=structuredClone(fixtures[name]);
    delete (name==='catalog'?missing.data.items[0]:missing.data)[key];
    assert.equal(validate(missing),false,`${name}: missing ${key} must fail`);
  }
}
const validateRelease=ajv.compile({$ref:`${schema.$id}#/definitions/release`});
for(const [key,value] of [['sha256','invalid'],['sizeBytes',0],['format','tar'],['downloadPath','https://other.invalid/package']]) {
  const invalid=structuredClone(fixtures.release); invalid.data.package[key]=value;
  assert.equal(validateRelease(invalid),false,`invalid package ${key} must fail`);
}

// Rust's actual serde types produce the domain objects checked against TypeScript below.
const rust=spawnSync('cargo',['test','--manifest-path','src-tauri/Cargo.toml','--no-default-features','--features','sds','--lib','sds::tests::catalog::contract_fixture_domain_shapes','--','--nocapture'],{cwd:root,encoding:'utf8',maxBuffer:8*1024*1024});
assert.equal(rust.status,0,`Rust contract consumer failed:\n${rust.stderr}\n${rust.stdout}`);
const match=rust.stdout.match(/SDS_CONTRACT_DOMAINS=(\{[^\r\n]+\})/);
assert.ok(match,'Rust contract test must run and emit actual serialized domain objects');
const domains=JSON.parse(match[1]);
const source=`import type {Page,SkillSummary,SkillDetail,Release,Taxonomy} from './src/features/sds/contracts';
const catalog:Page<SkillSummary>=${JSON.stringify(domains.catalog)};
const detail:SkillDetail=${JSON.stringify(domains.detail)};
const release:Release=${JSON.stringify(domains.release)};
const taxonomy:Taxonomy=${JSON.stringify(domains.taxonomy)};
void [catalog,detail,release,taxonomy];`;
const virtualFile=path.join(root,'sds-contract-check.ts');
const options={strict:true,noEmit:true,skipLibCheck:true,target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext,moduleResolution:ts.ModuleResolutionKind.Bundler};
const host=ts.createCompilerHost(options);
const originalGetSourceFile=host.getSourceFile.bind(host);
host.getSourceFile=(file,...args)=>path.resolve(file)===virtualFile?ts.createSourceFile(file,source,options.target,true):originalGetSourceFile(file,...args);
const program=ts.createProgram([virtualFile],options,host);
const diagnostics=ts.getPreEmitDiagnostics(program);
assert.equal(diagnostics.length,0,ts.formatDiagnosticsWithColorAndContext(diagnostics,{getCanonicalFileName:file=>file,getCurrentDirectory:()=>root,getNewLine:()=> '\n'}));
console.log('SDS contracts passed: six wire fixtures, compatibility/invalid metadata checks, Rust serde → TypeScript domain parity.');
