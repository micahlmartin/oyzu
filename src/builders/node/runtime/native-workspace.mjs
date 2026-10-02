// Native managers own workspace membership. This transport resolves an already
// provisioned JS entrypoint and invokes observation commands without a shell.
import {spawnSync} from 'node:child_process';
import {existsSync, readFileSync, realpathSync} from 'node:fs';
import {delimiter, isAbsolute, join, relative, resolve} from 'node:path';

export function entrypoint(manager, bin) {
  const candidates = [];
  for (const directory of (process.env.PATH ?? '').split(delimiter).filter(Boolean)) {
    const executable = join(directory, manager);
    if (existsSync(executable)) {
      const native = realpathSync(executable);
      if (native.replaceAll('\\', '/').endsWith('/'+bin)) candidates.push(native);
    }
    candidates.push(join(directory, 'node_modules', manager, bin),
      join(directory, '..', manager, bin));
  }
  const native = candidates.find(existsSync);
  if (!native) throw new Error(`Provision ${manager} with its native ${bin} entrypoint on PATH; Corepack acquisition is not invoked`);
  return [process.execPath, realpathSync(native)];
}

export function query(command, args) {
  const result = spawnSync(command[0], [...command.slice(1), ...args], {
    cwd:process.cwd(), encoding:'utf8', maxBuffer:4*1024*1024, timeout:120_000,
    env:{...process.env, COREPACK_ENABLE_NETWORK:'0', YARN_IGNORE_PATH:'1'},
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Native workspace observation failed: ${result.stderr}\n${result.stdout}`);
  return result.stdout;
}

export function member(name, location) {
  const root = realpathSync(process.cwd());
  const directory = realpathSync(resolve(location));
  const path = relative(root, directory).replaceAll('\\', '/');
  if (!path || isAbsolute(path) || path.split('/').includes('..')) throw new Error('Workspace member must be inside its root');
  const pkg = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
  if (pkg.name !== name) throw new Error('Native workspace identity differs from package.json');
  return {name, path};
}

export function emit(command, members, directory, separator) {
  members.sort((a,b)=>a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  process.stdout.write(JSON.stringify({command,members,directory,separator})+'\n');
}
