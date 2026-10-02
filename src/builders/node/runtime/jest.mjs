import {spawnSync} from 'node:child_process';
import {copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join} from 'node:path';
import {junit} from './jest-results.mjs';
import {testCommand} from './test-command.mjs';

const testReport = process.env.OYZU_TEST_REPORT;
const coverageReport = process.env.OYZU_COVERAGE_REPORT;
if (!testReport || !coverageReport) throw new Error('missing required Jest report destinations');
const [command, ...args] = await testCommand(process.argv.slice(2));
if (!command) throw new Error('missing native Jest invocation');
const temporary = mkdtempSync(join(tmpdir(), 'oyzu-jest-'));
try {
  const jsonPath = join(temporary, 'results.json');
  const coverageDirectory = join(temporary, 'coverage');
  const argv = [...args, '--ci', '--watch=false', '--watchAll=false', '--maxWorkers=2',
    '--json', `--outputFile=${jsonPath}`, '--coverage=true', '--coverageReporters=lcovonly',
    `--coverageDirectory=${coverageDirectory}`];
  const result = spawnSync(command, argv, {stdio: 'inherit'});
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
  const lcov = join(coverageDirectory, 'lcov.info');
  if (existsSync(lcov)) {
    mkdirSync(dirname(coverageReport), {recursive: true});
    copyFileSync(lcov, coverageReport);
  }
  if (existsSync(jsonPath)) {
    const native = JSON.parse(readFileSync(jsonPath, 'utf8'));
    mkdirSync(dirname(testReport), {recursive: true});
    writeFileSync(testReport, junit(native));
    if (!native.success && process.exitCode === 0) process.exitCode = 1;
  }
  if (!existsSync(jsonPath) || !existsSync(lcov)) process.exitCode ||= 1;
} finally {
  rmSync(temporary, {recursive: true, force: true});
}
