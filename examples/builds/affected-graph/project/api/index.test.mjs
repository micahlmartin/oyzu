import {test} from "node:test";
import assert from "node:assert/strict";
import {output} from "./index.mjs";
test("api output",()=>assert.equal(output,"hello api"));
