// Installed native dependencies and admitted host environment; no acquisition
// or source/version projection is performed for a direct test invocation.
import {npmScriptCommand} from './npm-native.mjs';
import {testWorkspace} from './npm-workspace-testing.mjs';

const [encoded, reportRoot, ...extra] = process.argv.slice(2);
const spec = JSON.parse(encoded);
const script = (name, member) => npmScriptCommand(['run', name,
  ...(member ? ['--workspace', member.name] : []), '--']);
process.exitCode = testWorkspace(spec, {root:process.cwd(), reportRoot, script, extra});
