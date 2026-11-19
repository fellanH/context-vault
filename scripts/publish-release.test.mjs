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
 if(args[0]==='run'&&args[1]==='view') {
  const fail=(process.env.CV_TEST_FAIL_JOB||'').split(',');
  const names=['build aarch64-apple-darwin','build x86_64-unknown-linux-gnu','publish GitHub Release (install.sh / install.ps1 source)'];
  console.log(JSON.stringify({jobs:names.map(name=>({name,conclusion:fail.some(f=>f&&name.startsWith(f))?'failure':'success'}))}));
 } else if(args[0]==='run'&&args[1]==='list') console.log(JSON.stringify([{databaseId:1,status:'completed',conclusion:process.env.CV_TEST_OLD_RUN||'success'}]));
 else if(args[1]==='download') {
  const dir=args[args.indexOf('--dir')+1];
  const files=['aarch64-apple-darwin.tar.gz','x86_64-apple-darwin.tar.gz','aarch64-unknown-linux-gnu.tar.gz','x86_64-unknown-linux-gnu.tar.gz','x86_64-pc-windows-msvc.zip'].map(f=>'context-vault-'+f);
  const hash=createHash('sha256').update('fixture').digest('hex');
  for(const file of files)writeFileSync(join(dir,file),'fixture');
  writeFileSync(join(dir,'SHA256SUMS'), files.map(f=>hash+'  '+f).join('\\n'));
  if(process.env.CV_TEST_CORRUPT)writeFileSync(join(dir,files[0]),'corrupt');
 } else if(args[1]==='view') {
  if(args.includes('example/public')&&!process.env.CV_TEST_MIRROR_EXISTS) process.exit(1);
  console.log('{"body":"Fixture release notes"}');
 }
}
`;
  for (const tool of ['git', 'gh', 'scaffold']) writeFileSync(join(root, 'bin', tool), stub, { mode: 0o755 });
  const log = join(root, 'calls.log');
  return {
    run(mode, extra = {}) {
      const result = spawnSync(process.execPath, [join(root, 'scripts/publish-release.mjs'), '4.0.1', mode], {
        cwd: root, encoding: 'utf8', env: { ...process.env, PATH: `${join(root, 'bin')}:${process.env.PATH}`, CV_TEST_LOG: log, CONTEXT_VAULT_SKIP_LOCAL_INSTALL: '1', ...extra },
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
const gh = (calls, sub) => calls.findIndex(c => c.tool === 'gh' && c.args[0] === 'release' && c.args[1] === sub);
const madeLatest = calls => calls.some(c => c.tool === 'gh' && c.args[1] === 'edit' && c.args.includes('--latest'));
test('the mirror release is a pre-release until its CI assets verify, then Latest', t => {
  const { result, calls } = fixture(t).run('--apply');
  assert.equal(result.status, 0, result.stderr);
  const gate = calls.findIndex(c => c.tool === 'scaffold');
  const create = gh(calls, 'create'), download = gh(calls, 'download');
  const latest = calls.findIndex(c => c.tool === 'gh' && c.args[1] === 'edit' && c.args.includes('--latest'));
  assert.ok(gate >= 0 && create > gate && download > create && latest > download);
  assert.deepEqual(calls[gate].args.slice(-2), ['--rev', 'v4.0.1']);
  assert.ok(calls[create].args.includes('example/public') && calls[create].args.includes('--prerelease'));
  assert.equal(calls[create].args.filter(a => /context-vault-.*\.(tar\.gz|zip)$/.test(a)).length, 0);
  assert.ok(calls[download].args.includes('example/public'));
  assert.ok(calls[latest].args.includes('--prerelease=false'));
  assert.ok(calls.some(c => c.tool === 'gh' && c.args[0] === 'run' && c.args.includes('example/public')));
});
test('a rerun edits the existing mirror release instead of creating one', t => {
  const { result, calls } = fixture(t).run('--apply', { CV_TEST_MIRROR_EXISTS: '1' });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(gh(calls, 'create'), -1);
  assert.ok(madeLatest(calls));
});
test('confcheck refusal prevents public release creation', t => {
  const { result, calls } = fixture(t).run('--apply', { CV_TEST_REFUSE_EXPORT: '1' });
  assert.notEqual(result.status, 0);
  assert.equal(gh(calls, 'create'), -1);
});
test('corrupt mirror assets leave the release a pre-release', t => {
  const { result, calls } = fixture(t).run('--apply', { CV_TEST_CORRUPT: '1' });
  assert.notEqual(result.status, 0);
  assert.ok(!madeLatest(calls));
});
test('a failed mirror build leaves the release a pre-release and says so', t => {
  const { result, calls } = fixture(t).run('--apply', { CV_TEST_FAIL_JOB: 'build x86_64' });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /build x86_64-unknown-linux-gnu failure/);
  assert.match(result.stderr, /stays a pre-release/);
  assert.equal(gh(calls, 'download'), -1);
  assert.ok(!madeLatest(calls));
});
test('a rerun after a failed mirror build reruns its failed jobs, then goes Latest', t => {
  const { result, calls } = fixture(t).run('--apply', { CV_TEST_MIRROR_EXISTS: '1', CV_TEST_OLD_RUN: 'failure' });
  assert.equal(result.status, 0, result.stderr);
  const rerun = calls.findIndex(c => c.tool === 'gh' && c.args[1] === 'rerun' && c.args.includes('--failed'));
  const list = calls.find(c => c.tool === 'gh' && c.args[1] === 'list');
  assert.ok(list.args.includes('--branch') && list.args.includes('v4.0.1') && list.args.includes('push'));
  assert.ok(rerun >= 0 && calls.some((c, i) => i > rerun && c.tool === 'gh' && c.args[1] === 'watch'));
  assert.ok(madeLatest(calls));
});
