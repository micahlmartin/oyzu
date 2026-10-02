// `plan` is a typed, bounded native workspace plan supplied by the Rust adapter.
const {spawnSync} = require('node:child_process');
for (const step of plan.steps) {
  if (!step.build) {
    console.log(`${step.name}: no declared build script; compilation not requested`);
    continue;
  }
  const [node, cli] = plan.command;
  const result = spawnSync(node, [cli, 'run', 'build', '--workspace', step.name, '--', ...process.argv.slice(1)], {stdio:'inherit'});
  if (result.error) throw result.error;
  if (result.status !== 0) {
    process.exitCode = result.status ?? 1;
    break;
  }
}
