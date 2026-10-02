import {inventory} from './yarn-workspaces.mjs';
import {entrypoint, query, member, emit} from './native-workspace.mjs';

const command = [...entrypoint('yarn', 'bin/yarn.js'), '--offline', '--non-interactive', '--ignore-path'];
const version = query(command, ['--version']).trim();
if (!version.startsWith('1.')) throw new Error('Workspace tests currently require provisioned Yarn Classic 1');
const native = inventory(query(command, ['--json', 'workspaces', 'info']));
emit(command, Object.entries(native).map(([name, details]) => member(name, details.location)), '--cwd', []);
