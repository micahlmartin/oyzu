import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync, writeFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {readLock, verify} from '../src/builders/node/runtime/lock.mjs';

test('npm lock admission refuses uncaptured input kinds and integrity downgrade', () => {
  const root = mkdtempSync(join(tmpdir(), 'oyzu-npm-lock-'));
  try {
    const bytes = Buffer.from('locked bytes');
    const integrity = `sha512-${createHash('sha512').update(bytes).digest('base64')}`;
    const original = {version:'1.0.0', resolved:'https://registry.npmjs.org/example/-/example-1.0.0.tgz', integrity};
    const write = (entry, path='node_modules/example', filename='package-lock.json') => {
      writeFileSync(join(root,filename), JSON.stringify({lockfileVersion:3,packages:{'':{},[path]:entry}}));
    };
    write(original);
    assert.equal(readLock(root).packages[0].name, 'example');
    assert.equal(verify(bytes, integrity), createHash('sha256').update(bytes).digest('hex'));
    assert.throws(()=>verify(Buffer.from('tampered'), integrity), /integrity mismatch/);
    for (const mutation of [{link:true}, {inBundle:true}, {resolved:'file:../outside'},
      {resolved:'git+https://example.invalid/repo.git'}, {resolved:'https://user:token@example.invalid/a.tgz'},
      {resolved:'https://example.invalid/a.tgz?token=secret'}, {integrity:'sha1-weak'}]) {
      write({...original,...mutation});
      assert.throws(()=>readLock(root));
    }
    for (const path of ['../outside','node_modules/../escape','/absolute','node_modules/hello\\world']) {
      write(original,path);
      assert.throws(()=>readLock(root));
    }
    write(original,'node_modules/@scope/example/node_modules/child');
    assert.equal(readLock(root).packages[0].name, 'child');
    write({...original,version:'2.0.0'},'node_modules/example','npm-shrinkwrap.json');
    assert.equal(readLock(root).packages[0].version, '2.0.0');
  } finally { rmSync(root,{recursive:true,force:true}); }
});
