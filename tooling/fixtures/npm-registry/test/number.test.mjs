import test from "node:test";
import assert from "node:assert/strict";
import { numeric } from "../src/number.mjs";
test("uses a captured third-party dependency", () => {
  assert.equal(numeric("42"), true);
  assert.equal(numeric("Oyzu"), false);
});
