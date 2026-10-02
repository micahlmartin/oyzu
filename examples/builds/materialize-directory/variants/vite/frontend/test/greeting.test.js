import { test } from "node:test";
import assert from "node:assert/strict";
import { greeting } from "../src/greeting.js";

test("greets the requested user", () => {
  assert.equal(greeting("Oyzu"), "Hello, Oyzu!");
});
