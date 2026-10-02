const assert = require("node:assert/strict");
const { value } = require("../index.js");

it("root only", () => assert.equal(value, 42));
