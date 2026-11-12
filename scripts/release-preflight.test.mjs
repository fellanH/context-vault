import test from 'node:test';
import assert from 'node:assert/strict';
import { validateReleaseTarget } from './release-preflight.mjs';

const config = {
  manifest: '[publish]\nto = "https://github.com/example/public.git"\n',
  installSh: 'REPO="example/public"',
  installPs1: "$Repo = 'example/public'",
  origin: 'https://github.com/example/public.git',
};
test('matching targets preserve the existing single-repository release', () => {
  assert.equal(validateReleaseTarget(config), 'example/public');
});
test('a proposed mirror mismatch refuses publication', () => {
  assert.throws(() => validateReleaseTarget({ ...config, manifest: '[publish]\nto = "https://github.com/example/proposed.git"' }), /destinations disagree/);
});
test('a private-source release refuses until public asset publication is wired', () => {
  assert.throws(() => validateReleaseTarget({ ...config, origin: 'git@github.com:example/private.git' }), /current CI/);
});
test('missing configuration fails closed', () => {
  assert.throws(() => validateReleaseTarget({ ...config, manifest: '' }), /Cannot resolve/);
});

test('a comment naming the publish section does not hide the real destination', () => {
  assert.equal(validateReleaseTarget({ ...config, manifest: '# [publish] describes the export\n' + config.manifest }), 'example/public');
});

test('private-source publication is permitted only with the gated mirror publisher wired', () => {
  assert.equal(validateReleaseTarget({ ...config, origin: 'https://github.com/example/private.git', mirrorPublisherConfigured: true }), 'example/private');
});
