import assert from 'node:assert/strict';
import { test } from 'node:test';
import { greeting } from '@oyzu-fixture/greeting';
test('private dependency', () => assert.equal(greeting(), 'hello'));
