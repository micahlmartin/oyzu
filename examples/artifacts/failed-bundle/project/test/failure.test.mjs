import {test} from "node:test";
import assert from "node:assert/strict";
test("controlled failure",()=>assert.notEqual(process.env.EXAMPLE_FAIL,"1"));
