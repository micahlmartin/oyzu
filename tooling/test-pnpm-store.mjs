// Export-boundary rejection checks; native install compatibility is exercised
// separately by test-node-registry-acquisition.py against provisioned pnpm.
import assert from 'node:assert/strict';
import {mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import test from 'node:test';
import {exportStore} from '../src/builders/node/runtime/pnpm-store.mjs';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'oyzu-pnpm-store-'));
  t.after(() => rmSync(root, {recursive:true, force:true}));
  const output = join(root, 'out'), temporary = join(root, 'private');
  mkdirSync(output); mkdirSync(join(temporary, 'store'), {recursive:true});
  return {root, output, temporary, inventory:{version:'10.11.0', packages:[]}};
}

test('unqualified pnpm versions cannot borrow store format assumptions', t => {
  const context = fixture(t);
  context.inventory.version = '11.0.0';
  assert.throws(() => exportStore(context), /qualified pnpm/);
});

test('configuration files and side-effect metadata cannot enter the exported store', t => {
  const context = fixture(t);
  writeFileSync(join(context.temporary, 'store', '.npmrc'), 'fixture-only');
  assert.throws(() => exportStore(context), /unsupported native pnpm store file/);
  const other = fixture(t);
  const folder = join(other.temporary, 'store/v10/index/aa');
  mkdirSync(folder, {recursive:true});
  writeFileSync(join(folder, 'b'.repeat(62) + '-fixture@1.0.0.json'), JSON.stringify({
    name:'fixture', version:'1.0.0', files:{}, sideEffects:{fixture:{}},
  }));
  assert.throws(() => exportStore(other), /unbuilt native file index/);
});

test('directory links cannot include unrelated preparation state', t => {
  const context = fixture(t);
  const outside = join(context.root, 'unrelated');
  mkdirSync(outside); writeFileSync(join(outside, 'canary'), 'keep-private');
  mkdirSync(join(context.temporary, 'store/v10/files'), {recursive:true});
  symlinkSync(outside, join(context.temporary, 'store/v10/files/aa'), process.platform === 'win32' ? 'junction' : 'dir');
  assert.throws(() => exportStore(context), /rejects links/);
  assert.equal(readFileSync(join(outside, 'canary'), 'utf8'), 'keep-private');
});
