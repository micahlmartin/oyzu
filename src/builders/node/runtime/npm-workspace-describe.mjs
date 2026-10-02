// This helper is invoked only after an explicit development task request.
import {readFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {npmCommand} from './npm-native.mjs';
import {members, graph} from './npm-workspaces.mjs';

const root = process.cwd();
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
const workspaces = await graph(root, await members(root, pkg));
// Resolve only the executable/CLI prefix. Development scripts retain normal
// npm configuration; captured builds use their separate offline preparation.
console.log(JSON.stringify({command:npmCommand([], tmpdir()).slice(0,2), workspaces}));
