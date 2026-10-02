// Bind native commands and library integration to the same provisioned npm.
import {spawnSync} from 'node:child_process';
import {accessSync, constants, realpathSync} from 'node:fs';
import {createRequire} from 'node:module';
import {delimiter, dirname, join} from 'node:path';

function entrypoint() {
  if (process.platform === 'win32') return join(dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js');
  for (const directory of (process.env.PATH ?? '').split(delimiter)) {
    if (!directory) continue;
    const candidate = join(directory, 'npm');
    try {
      accessSync(candidate, constants.X_OK);
      const file = realpathSync(candidate);
      if (!file.endsWith('/npm-cli.js')) throw new Error('Provisioned npm must expose its native npm-cli.js entrypoint');
      return file;
    } catch (error) {
      if (!['ENOENT', 'EACCES', 'ENOTDIR'].includes(error.code)) throw error;
    }
  }
  throw new Error('No provisioned npm entrypoint found');
}

const cli = entrypoint();
export const nativeRequire = createRequire(cli);
// Host scripts retain native configuration and lifecycle behavior. Resolve the
// JavaScript entrypoint so Windows does not need to spawn a .cmd shim or shell.
export function npmScriptCommand(args) {
  return [process.execPath, cli, ...args];
}
export function npmCommand(args, cache) {
  return [process.execPath, cli, '--offline', '--audit=false', '--fund=false',
    '--update-notifier=false', '--engine-strict=true', '--force=false', '--cache', cache,
    '--userconfig', join(cache, 'user.npmrc'), '--globalconfig', join(cache, 'global.npmrc'), ...args];
}
export function npm(args, workspace, cache) {
  const [command, ...argv] = npmCommand(args, cache);
  const result = spawnSync(command, argv, {
    cwd: workspace, encoding: 'utf8', timeout: 240_000, maxBuffer: 8 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`npm ${args[0]} failed (${result.status}):\n${result.stdout}\n${result.stderr}`);
  return result.stdout;
}
