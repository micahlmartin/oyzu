const assert = require("node:assert/strict");
const { value } = require("../index.js");

it("implicit member only", () => assert.equal(value, 11));
