// Invoke the captured native CLI with a private configuration overlay. Native
// configuration is evaluated only inside the test action, never in discovery.
import {spawnSync} from 'node:child_process';
import {copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {tmpdir} from 'node:os';
import {dirname, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {testCommand} from './test-command.mjs';

const testReport = process.env.OYZU_TEST_REPORT;
const coverageReport = process.env.OYZU_COVERAGE_REPORT;
if (!testReport || !coverageReport) throw new Error('missing required Vitest report destinations');
const [command, ...args] = await testCommand(process.argv.slice(2));
if (!command) throw new Error('missing native Vitest invocation');
const workspace = process.cwd();
const require = createRequire(join(workspace, 'package.json'));
const version = JSON.parse(readFileSync(require.resolve('vitest/package.json'), 'utf8')).version;
if (!version.startsWith('5.')) throw new Error(`Vitest ${version}: this adapter requires the Vitest 5 native configuration contract`);
const temporary = mkdtempSync(join(tmpdir(), 'oyzu-vitest-'));
try {
  const junit = join(temporary, 'junit.xml');
  const coverage = join(temporary, 'coverage');
  const overlay = join(temporary, 'config.mjs');
  const config = ['vitest.config', 'vite.config'].flatMap(base =>
    ['ts','mts','cts','js','mjs','cjs'].map(extension => join(workspace, `${base}.${extension}`)))
    .find(existsSync);
  const settings = {workspace, config, junit, coverage, version};
  writeFileSync(overlay, `
import {loadConfigFromFile} from ${JSON.stringify(pathToFileURL(require.resolve('vite')).href)};
import {createRequire} from 'node:module';
import {join} from 'node:path';
const settings = ${JSON.stringify(settings)};
export default async function(environment) {
  const loaded = settings.config ? await loadConfigFromFile(environment, settings.config, settings.workspace) : null;
  const config = loaded?.config ?? {};
  const test = config.test ?? {};
  const coverage = test.coverage ?? {};
  const provider = coverage.provider ?? 'v8';
  if (!['v8','istanbul'].includes(provider)) throw new Error('unsupported Vitest coverage provider ' + provider);
  const require = createRequire(join(settings.workspace, 'package.json'));
  let installed;
  try { installed = require('@vitest/coverage-' + provider + '/package.json'); }
  catch { throw new Error('Vitest coverage provider must be captured before testing: @vitest/coverage-' + provider); }
  if (installed.version !== settings.version) throw new Error('Vitest and coverage provider versions must match');
  const reporters = test.reporters == null ? require('vitest/config').configDefaults.reporters : Array.isArray(test.reporters) ? test.reporters : [test.reporters];
  const coverageReporters = coverage.reporter == null ? [] : Array.isArray(coverage.reporter) ? coverage.reporter : [coverage.reporter];
  return {...config, test: {...test,
    reporters: [...reporters, ['junit', {outputFile: settings.junit}]],
    coverage: {...coverage, enabled: true, reportOnFailure: true, reportsDirectory: settings.coverage,
      reporter: [...coverageReporters.filter(r => !['lcov','lcovonly'].includes(Array.isArray(r) ? r[0] : r)), 'lcovonly']}
  }};
}
`);
  const result = spawnSync(command, [...args, '--run', '--watch=false', '--maxWorkers=2',
    `--config=${overlay}`, '--configLoader=native'], {stdio:'inherit', env:{...process.env, CI:'true'}});
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
  for (const [source, destination] of [[junit, testReport], [join(coverage,'lcov.info'), coverageReport]]) {
    if (existsSync(source)) {
      mkdirSync(dirname(destination), {recursive:true});
      copyFileSync(source, destination);
    } else process.exitCode ||= 1;
  }
} finally { rmSync(temporary, {recursive:true, force:true}); }
