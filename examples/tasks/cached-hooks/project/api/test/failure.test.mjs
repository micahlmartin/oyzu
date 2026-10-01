import { test } from "node:test";
import assert from "node:assert/strict";
import { appendFileSync, mkdirSync } from "node:fs";
test("controlled failure",()=>{mkdirSync(".events",{recursive:true});appendFileSync(".events/order.txt","main\n");assert.notEqual(process.env.FAILURE_POINT,"main");});
