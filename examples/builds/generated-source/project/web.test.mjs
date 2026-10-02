import { test } from "node:test";
import assert from "node:assert/strict";
import { message } from "./generated/message.mjs";
test("web consumes generated data", () =>
  assert.equal(message, "Hello, Oyzu!"));
