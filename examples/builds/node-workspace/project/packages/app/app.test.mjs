import {test} from "node:test";
import assert from "node:assert/strict";
import {label} from "@oyzu-example/shared";
test("consumes shared",()=>assert.equal(label,"shared"));
