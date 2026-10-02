import { test } from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
test("calls a test-owned loopback service", async () => {
  const server = createServer((req, res) => {
    res.setHeader("content-type", "application/json");
    res.end(JSON.stringify({ status: "ok" }));
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  try {
    const response = await fetch(
      "http://127.0.0.1:" + server.address().port + "/health",
    );
    assert.deepEqual(await response.json(), { status: "ok" });
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
