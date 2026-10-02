const assert = require("node:assert/strict");
const { value } = require("../index.js");

it("scripted member only", () => assert.equal(value, 23));
