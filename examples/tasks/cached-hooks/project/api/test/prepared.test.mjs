import {test} from "node:test";
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
test("consumes the pre-hook output",()=>{
 const input=JSON.parse(readFileSync("generated/input.json","utf8"));
 assert.equal(typeof input.message,"string");
 assert.ok(input.message.length>0);
});
