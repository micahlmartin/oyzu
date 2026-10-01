import {test} from "node:test";
import assert from "node:assert/strict";
import {output} from "./index.mjs";
test("web output",()=>assert.equal(output,"hello web"));
