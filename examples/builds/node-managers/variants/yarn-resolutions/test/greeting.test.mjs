import { test } from "node:test";
import assert from "node:assert/strict";
import { greeting } from "../src/greeting.mjs";
import isOdd from "is-odd";
import colors from "@colors/colors/safe.js";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
test("keeps the direct version and overrides only the selected dependency", () => {
  assert.equal(require("is-number/package.json").version, "6.0.0");
  const fromOdd = createRequire(require.resolve("is-odd"));
  assert.equal(fromOdd("is-number/package.json").version, "7.0.0");
});
test("uses a scoped captured dependency", () =>
  assert.equal(colors.strip("hello"), "hello"));
test("uses a captured transitive dependency", () =>
  assert.equal(isOdd(3), true));
test("greets the requested name", () =>
  assert.equal(greeting("Oyzu"), "Hello, Oyzu!"));
test("has a useful default", () => assert.equal(greeting(), "Hello, world!"));
