// `plan` is a typed, bounded native workspace plan supplied by the Rust adapter.
const {spawnSync} = require('node:child_process');
const {resolve} = require('node:path');
const {createRequire} = require('node:module');
const extra = process.argv.slice(1);
if (extra.length && plan.steps.some(step => step.operation.kind === 'quality')) {
  throw new Error('Implicit workspace quality checks do not accept extra arguments; define a native script or an argv task');
}
async function run() {
  const scope = plan.testScope ? await import(`data:text/javascript;base64,${Buffer.from(plan.testScope).toString('base64')}`) : null;
  for (const step of plan.steps) {
    const operation = step.operation;
    if (operation.kind === 'no-compilation') {
      console.log(`${step.name}: no declared build script; compilation not requested`);
      continue;
    }
    const [node, cli] = plan.command;
    let args, cwd = process.cwd(), env = process.env;
    if (operation.kind === 'script') {
      args = [cli, 'run', operation.script, '--workspace', step.name, '--', ...extra];
    } else if (operation.kind === 'quality') {
      args = ['--input-type=module', '-e', plan.quality, operation.checker === 'biome' ? `biome-${plan.stage}` : plan.stage];
      cwd = resolve(cwd, step.path);
      env = {...env, OYZU_NODE_TEST_FRAMEWORK:operation.framework, OYZU_NODE_QUALITY_EXCLUDE:JSON.stringify(operation.excludes)};
    } else if (operation.kind === 'test') {
      cwd = resolve(cwd, step.path);
      const members = operation.excludes.map(path => ({path}));
      if (operation.framework === 'node-test') {
        args = ['--test', ...extra, ...scope.nodeTests(cwd, members)];
      } else {
        const native = createRequire(resolve(cwd, 'package.json'));
        const jest = operation.framework === 'jest';
        args = [...scope.frameworkCommand(operation.framework, cwd).slice(1), ...extra,
          ...scope.frameworkArguments(operation.framework, cwd, members, jest ? native('jest/package.json').version : undefined)];
      }
    } else throw new Error('Unknown workspace development operation');
    const result = spawnSync(node, args, {stdio:'inherit', cwd, env});
    if (result.error) throw result.error;
    if (result.status !== 0) {
      process.exitCode = result.status ?? 1;
      if (plan.stage === 'build') break;
    }
  }
}
run().catch(error => { console.error(error); process.exitCode = 1; });
