import { spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname } from "node:path";

const junit = process.env.OYZU_TEST_REPORT ?? "reports/tests.xml";
const coverage = process.env.OYZU_COVERAGE_REPORT ?? "reports/coverage.lcov";
for (const path of [junit, coverage])
  mkdirSync(dirname(path), { recursive: true });
const result = spawnSync(
  process.execPath,
  [
    "--test",
    "--experimental-test-coverage",
    "--test-reporter=junit",
    `--test-reporter-destination=${junit}`,
    "--test-reporter=lcov",
    `--test-reporter-destination=${coverage}`,
  ],
  { stdio: "inherit" },
);
process.exitCode = result.status ?? 1;
