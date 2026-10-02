import {entrypoint, query, member, emit} from './native-workspace.mjs';

const command = [...entrypoint('yarn', 'bin/yarn.js'), '--offline', '--non-interactive', '--ignore-path'];
const version = query(command, ['--version']).trim();
if (!version.startsWith('1.')) throw new Error('Workspace tests currently require provisioned Yarn Classic 1');
const events = query(command, ['--json', 'workspaces', 'info']).trim().split(/\r?\n/).map(line => JSON.parse(line));
const logs = events.filter(event => event.type === 'log');
if (logs.length !== 1) throw new Error('Expected one native Yarn workspace inventory');
const native = JSON.parse(logs[0].data);
emit(command, Object.entries(native).map(([name, details]) => member(name, details.location)), '--cwd', []);
