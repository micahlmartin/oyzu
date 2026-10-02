import { test } from "node:test";
import assert from "node:assert/strict";
import { greeting } from "../src/greeting.mjs";
test("greets the requested name", () =>
  assert.equal(greeting("Oyzu"), "Hello, Oyzu!"));
test("has a useful default", () => assert.equal(greeting(), "Hello, world!"));
