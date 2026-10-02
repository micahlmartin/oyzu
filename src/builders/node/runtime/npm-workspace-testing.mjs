// One native test-composition implementation for captured and host workspaces.
import {spawnSync} from 'node:child_process';
import {mkdirSync} from 'node:fs';
import {createRequire} from 'node:module';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {nodeTests, frameworkArguments, frameworkCommand} from './npm-workspace-test-scope.mjs';

export function testWorkspace(spec, {root, reportRoot, script, extra = []}) {
  const runtime = dirname(fileURLToPath(import.meta.url));
  let failed = false;
  const invoke = (command, cwd, env) => {
    const result = spawnSync(command[0], command.slice(1), {cwd, env, stdio:'inherit'});
    if (result.error) throw result.error;
    return result.status ?? 1;
  };
  const rootProducer = {id:'root', path:'.', scripts:spec.rootScripts, framework:spec.rootFramework};
  const producers = spec.rootScripts.test ? [rootProducer]
    : [...spec.modules, ...(spec.rootArtifact ? [rootProducer] : [])];
  for (const member of producers) {
    const reports = join(reportRoot, member.id);
    mkdirSync(reports, {recursive:true});
    const env = {...process.env, OYZU_TEST_REPORT:producers.length === 1 && process.env.OYZU_TEST_REPORT || join(reports, 'junit.xml'), OYZU_COVERAGE_REPORT:producers.length === 1 && process.env.OYZU_COVERAGE_REPORT || join(reports, 'coverage.lcov')};
    const cwd = resolve(root, member.path);
    const excludes = member.id === 'root' ? spec.modules : member.testExcludes?.map(path => ({path}));
    if (!member.scripts.test && !excludes) throw new Error('Missing planned workspace test scope');
    let command;
    if (member.scripts.test) command = script('test', member.name ? member : undefined);
    else if (member.framework === 'node-test') command = [process.execPath, '--test'];
    else if (['jest', 'vitest', 'mocha'].includes(member.framework)) {
      command = frameworkCommand(member.framework, cwd);
    } else throw new Error(`No native test command for ${member.name}`);
    if (member.framework === 'node-test') {
      command.push(...extra);
      command.push(...spec.nodeTestArguments.map(v => v.replace('__OYZU_TEST_REPORT__', env.OYZU_TEST_REPORT).replace('__OYZU_COVERAGE_REPORT__', env.OYZU_COVERAGE_REPORT)));
      if (!member.scripts.test) command.push(...nodeTests(cwd, excludes));
    } else if (['jest', 'vitest', 'mocha'].includes(member.framework)) {
      command.push(...extra);
      if (!member.scripts.test) {
        const native = createRequire(join(cwd, 'package.json'));
        const version = member.framework === 'jest' ? native('jest/package.json').version : undefined;
        command.push(...frameworkArguments(member.framework, cwd, excludes, version));
      }
      command = [process.execPath, join(runtime, `${member.framework}.mjs`), ...command];
    }
    if (!['node-test', 'jest', 'vitest', 'mocha'].includes(member.framework)) command.push(...extra);
    // Native frameworks resolve native config and reporting dependencies from the
    // package they test, including when npm dispatches a member script.
    const status = invoke(command, member.scripts.test && !['jest', 'vitest', 'mocha'].includes(member.framework) ? root : cwd, env);
    if (status) failed = true;
  }
  return failed ? 1 : 0;
}
