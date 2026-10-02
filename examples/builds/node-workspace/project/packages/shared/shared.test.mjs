import { test } from "node:test";
import assert from "node:assert/strict";
import { label } from "./index.mjs";
test("label", () => assert.equal(label, "shared"));
