// Native command and membership were observed before hooks and frozen by Rust.
import {resolve} from 'node:path';
import {testWorkspace} from './workspace-testing.mjs';

const [encoded, reportRoot, ...extra] = process.argv.slice(2);
const spec = JSON.parse(encoded);
const root = process.cwd();
const script = (name, member) => [...spec.native.command, spec.native.directory,
  resolve(root, member?.path ?? '.'), 'run', name, ...spec.native.separator];
process.exitCode = testWorkspace(spec, {root,reportRoot,script,extra});
