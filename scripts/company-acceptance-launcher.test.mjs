import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { access, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { once } from 'node:events';

test('demo refuses an existing Company app instance before creating demo data or starting Store', { skip: process.platform !== 'win32', timeout: 20000 }, async () => {
  const root = await mkdtemp(join(tmpdir(), 'company-launcher-instance-'));
  const data = join(root, 'demo');
  // Own only this mutex handle; never stop or signal an existing user app.
  const holder = spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    "$mutex = [System.Threading.Mutex]::new($false, 'com.companyskills.manager-sim'); [Console]::WriteLine('MUTEX_READY'); [Console]::Out.Flush(); [Console]::ReadLine() | Out-Null; $mutex.Dispose()"],
  { windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
  let holderOutput = '';
  holder.stdout.on('data', chunk => holderOutput += chunk);
  let child;
  try {
    const deadline = Date.now() + 10000;
    while (!holderOutput.includes('MUTEX_READY')) {
      assert.equal(holder.exitCode, null);
      assert(Date.now() < deadline, 'Named mutex fixture did not start');
      await new Promise(done => setTimeout(done, 25));
    }
    child = spawn(process.execPath, ['scripts/company-acceptance-launcher.mjs', '--port', '0', '--data-root', data],
      { windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
    let output = '';
    child.stdout.on('data', chunk => output += chunk);
    child.stderr.on('data', chunk => output += chunk);
    const [code] = await once(child, 'exit');
    assert.equal(code, 1);
    assert.match(output, /đóng.*Skill Manager.*đang chạy/iu);
    assert.doesNotMatch(output, /SERVER_READY|APP_STARTED|APP_PROCESS_RUNNING/);
    await assert.rejects(access(data));
  } finally {
    if (child && child.exitCode === null) { const done = once(child, 'exit'); child.kill(); await done; }
    if (holder.exitCode === null) { const done = once(holder, 'exit'); holder.stdin.end('\n'); await done; }
    await rm(root, { recursive: true, force: true });
  }
});
