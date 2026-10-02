import { test } from "node:test";
import assert from "node:assert/strict";
import { greeting } from "../src/greeting.mjs";
import isOdd from "is-odd";
import { createRequire } from "node:module";
test("uses the native patched dependency", () => {
  const require = createRequire(import.meta.url);
  const fromOdd = createRequire(require.resolve("is-odd"));
  assert.equal(fromOdd("is-number").oyzuPatched, "native-patch-applied");
});
test("uses a captured transitive dependency", () =>
  assert.equal(isOdd(3), true));
test("greets the requested name", () =>
  assert.equal(greeting("Oyzu"), "Hello, Oyzu!"));
test("has a useful default", () => assert.equal(greeting(), "Hello, world!"));
