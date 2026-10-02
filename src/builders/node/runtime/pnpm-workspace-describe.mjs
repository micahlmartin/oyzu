import {realpathSync} from 'node:fs';
import {entrypoint, query, member, emit} from './native-workspace.mjs';

const command = [...entrypoint('pnpm', 'bin/pnpm.cjs'), '--config.manage-package-manager-versions=false'];
const version = query(command, ['--version']).trim();
if (!version.startsWith('10.')) throw new Error('Workspace tests currently require provisioned pnpm 10');
const root = realpathSync(process.cwd());
const native = JSON.parse(query(command, ['list', '--recursive', '--depth', '-1', '--json']));
if (!Array.isArray(native)) throw new Error('Expected native pnpm workspace list');
emit(command, native.filter(p => realpathSync(p.path) !== root).map(p => member(p.name, p.path)), '--dir', []);
