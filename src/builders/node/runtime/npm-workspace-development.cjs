// `plan` is a typed, bounded native workspace plan supplied by the Rust adapter.
const {spawnSync} = require('node:child_process');
const {resolve} = require('node:path');
const extra = process.argv.slice(1);
if (extra.length && plan.steps.some(step => step.operation.kind === 'quality')) {
  throw new Error('Implicit workspace quality checks do not accept extra arguments; define a native script or an argv task');
}
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
    args = ['--input-type=module', '-e', plan.quality, plan.stage];
    cwd = resolve(cwd, step.path);
    env = {...env, OYZU_NODE_TEST_FRAMEWORK:operation.framework, OYZU_NODE_QUALITY_EXCLUDE:JSON.stringify(operation.excludes)};
  } else throw new Error('Unknown workspace development operation');
  const result = spawnSync(node, args, {stdio:'inherit', cwd, env});
  if (result.error) throw result.error;
  if (result.status !== 0) {
    process.exitCode = result.status ?? 1;
    if (plan.stage === 'build') break;
  }
}
