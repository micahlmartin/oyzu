import test from 'node:test';
import assert from 'node:assert/strict';
import {junit} from '../src/builders/node/runtime/jest-results.mjs';

test('Jest conversion rejects unknown result shapes and preserves native failures', () => {
  assert.throws(()=>junit({}), /invalid Jest/);
  const assertion = {fullName:'native test', failureMessages:[], duration:1, status:'passed'};
  const suite = {name:'suite', assertionResults:[assertion], status:'passed'};
  const results = {success:true,testResults:[suite]};
  assert.match(junit(results), /tests="1" failures="0" errors="0" skipped="0"/);
  suite.status = 'focused';
  assert.match(junit(results), /tests="1" failures="0" errors="0" skipped="0"/);
  assertion.status = 'unknown';
  assert.throws(()=>junit(results), /unsupported Jest assertion/);
  assertion.status = 'passed';
  assertion.duration = -1;
  assert.throws(()=>junit(results), /duration/);
  assertion.duration = null;
  suite.status = 'failed';
  suite.message = 'failed & <unsafe>\u0000';
  results.success = false;
  assert.match(junit(results), /<error>failed &amp; &lt;unsafe&gt;\uFFFD<\/error>/);
  suite.assertionResults = [];
  assert.match(junit(results), /tests="1" failures="0" errors="1"/);
  assert.match(junit({success:true,testResults:[]}), /tests="0" failures="0" errors="0" skipped="0"/);
});
