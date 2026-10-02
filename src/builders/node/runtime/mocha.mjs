// Run the captured native manager/Mocha command under native c8. No acquisition
// occurs here. Fresh private destinations prevent stale reports surviving failure.
import {spawnSync} from 'node:child_process';
import {copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync} from 'node:fs';
import {createRequire} from 'node:module';
import {tmpdir} from 'node:os';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {testCommand} from './test-command.mjs';

const test = process.env.OYZU_TEST_REPORT, coverage = process.env.OYZU_COVERAGE_REPORT;
const [command, ...args] = await testCommand(process.argv.slice(2));
if (!test || !coverage || !command) throw new Error('Missing Mocha command/report contract');
const project = createRequire(join(process.cwd(), 'package.json'));
const version = project('mocha/package.json').version;
if (!version.startsWith('11.')) throw new Error(`Mocha ${version}: this adapter requires Mocha 11`);
let c8;
try { c8 = project.resolve('c8/package.json'); }
catch (error) {
  const pkg = project('./package.json');
  if (error.code !== 'MODULE_NOT_FOUND' || ['dependencies','devDependencies','optionalDependencies'].some(k => Object.hasOwn(pkg[k] ?? {}, 'c8'))) throw error;
  if (!process.env.OYZU_NODE_REPORTING_HOME) throw new Error('No provisioned c8 coverage tool');
  c8 = createRequire(join(resolve(process.env.OYZU_NODE_REPORTING_HOME), 'package.json')).resolve('c8/package.json');
}
if (!JSON.parse(readFileSync(c8, 'utf8')).version.startsWith('10.')) throw new Error('Mocha coverage adapter requires c8 10');
const temporary = mkdtempSync(join(tmpdir(), 'oyzu-mocha-'));
try {
  const junit = join(temporary, 'junit.xml');
  const reportDirectory = join(temporary, 'reports');
  const reporter = fileURLToPath(new URL('./mocha-reporter.cjs', import.meta.url));
  const result = spawnSync(process.execPath, [join(dirname(c8), 'bin/c8.js'), '--reporter=lcovonly',
    `--reports-dir=${reportDirectory}`, `--temp-directory=${join(temporary,'v8')}`,
    command, ...args, '--watch=false', '--reporter', reporter],
    {stdio:'inherit', env:{...process.env, OYZU_MOCHA_JUNIT:junit}});
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
  for (const [source, destination] of [[junit,test], [join(reportDirectory,'lcov.info'),coverage]]) {
    if (existsSync(source)) {
      mkdirSync(dirname(destination), {recursive:true});
      copyFileSync(source, destination);
    } else process.exitCode ||= 1;
  }
} finally { rmSync(temporary, {recursive:true, force:true}); }
