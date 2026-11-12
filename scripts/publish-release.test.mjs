import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, copyFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// Real orchestration, with local executable substitutes for Git, gh and scaffold.
function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'cv-publish-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'scripts')); mkdirSync(join(root, 'bin'));
  for (const file of ['publish-release.mjs', 'release-preflight.mjs', 'release-assets.mjs']) {
    copyFileSync(fileURLToPath(new URL(file, import.meta.url)), join(root, 'scripts', file));
  }
  writeFileSync(join(root, 'domain.toml'), '[publish]\nto = "https://github.com/example/public.git"\nbranch = "main"\n');
  writeFileSync(join(root, 'install.sh'), 'REPO="example/public"');
  writeFileSync(join(root, 'install.ps1'), "$Repo = 'example/public'");
  writeFileSync(join(root, 'package.json'), '{"version":"4.0.1"}');
  const stub = `#!${process.execPath}
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { createHash } from 'node:crypto';
const tool=basename(process.argv[1]), args=process.argv.slice(2);
appendFileSync(process.env.CV_TEST_LOG, JSON.stringify({tool,args})+'\\n');
if(tool==='git') {
 if(args[0]==='remote') console.log('https://github.com/example/private.git');
 else if(args[0]==='rev-parse') console.log('fixture-sha');
 else if(args[0]==='show') console.log(readFileSync(args[1].split(':')[1],'utf8'));
} else if(tool==='scaffold') {
 if(process.env.CV_TEST_REFUSE_EXPORT) process.exit(2);
} else if(tool==='gh') {
 if(args[0]==='run') console.log('[{"databaseId":1,"status":"completed","conclusion":"success"}]');
 else if(args[1]==='download') {
  const dir=args[args.indexOf('--dir')+1];
  const files=['aarch64-apple-darwin.tar.gz','x86_64-apple-darwin.tar.gz','aarch64-unknown-linux-gnu.tar.gz','x86_64-unknown-linux-gnu.tar.gz','x86_64-pc-windows-msvc.zip'].map(f=>'context-vault-'+f);
  const hash=createHash('sha256').update('fixture').digest('hex');
  for(const file of files)writeFileSync(join(dir,file),'fixture');
  writeFileSync(join(dir,'SHA256SUMS'), files.map(f=>hash+'  '+f).join('\\n'));
  if(process.env.CV_TEST_CORRUPT)writeFileSync(join(dir,files[0]),'corrupt');
 } else if(args[1]==='view') {
  if(args.includes('example/public')) process.exit(1);
  console.log('{"body":"Fixture release notes"}');
 }
}
`;
  for (const tool of ['git', 'gh', 'scaffold']) writeFileSync(join(root, 'bin', tool), stub, { mode: 0o755 });
  const log = join(root, 'calls.log');
  return {
    run(mode, extra = {}) {
      const result = spawnSync(process.execPath, [join(root, 'scripts/publish-release.mjs'), '4.0.1', mode], {
        cwd: root, encoding: 'utf8', env: { ...process.env, PATH: `${join(root, 'bin')}:${process.env.PATH}`, CV_TEST_LOG: log, ...extra },
      });
      const calls = readFileSync(log, 'utf8').trim().split('\n').map(JSON.parse);
      return { result, calls };
    },
  };
}
test('check mode makes no gh or public-export calls', t => {
  const { result, calls } = fixture(t).run('--check');
  assert.equal(result.status, 0, result.stderr);
  assert.ok(calls.every(c => c.tool === 'git'));
});
test('verified assets and the gated tag export precede public release creation', t => {
  const { result, calls } = fixture(t).run('--apply');
  assert.equal(result.status, 0, result.stderr);
  const gate = calls.findIndex(c => c.tool === 'scaffold');
  const publish = calls.findIndex(c => c.tool === 'gh' && c.args[1] === 'create');
  assert.ok(gate >= 0 && publish > gate);
  assert.deepEqual(calls[gate].args.slice(-2), ['--rev', 'v4.0.1']);
  assert.ok(calls[publish].args.includes('example/public'));
  assert.equal(calls[publish].args.filter(a => /context-vault-.*\.(tar\.gz|zip)$/.test(a)).length, 5);
});
test('confcheck refusal prevents public release creation', t => {
  const { result, calls } = fixture(t).run('--apply', { CV_TEST_REFUSE_EXPORT: '1' });
  assert.notEqual(result.status, 0);
  assert.ok(!calls.some(c => c.tool === 'gh' && c.args[1] === 'create'));
});
test('corrupt private assets prevent even the public export', t => {
  const { result, calls } = fixture(t).run('--apply', { CV_TEST_CORRUPT: '1' });
  assert.notEqual(result.status, 0);
  assert.ok(!calls.some(c => c.tool === 'scaffold'));
});
