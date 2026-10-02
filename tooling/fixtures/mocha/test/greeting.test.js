import assert from "node:assert/strict";
import { greeting } from "../src/greeting.js";

describe("native Mocha", () => {
  it("greets <Oyzu> & friends", () => {
    assert.equal(greeting("Oyzu"), "Hello, Oyzu!");
  });
  it.skip("optional test", () => {});
});
