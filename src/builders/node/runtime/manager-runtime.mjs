// Shared lifecycle only. Each native manager supplies its own command/arguments.
import {spawnSync} from 'node:child_process';
import {mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

export async function run(profile) {
  const [mode, output = mode === 'acquire' ? '/out' : '/dependencies', workspace = '/workspace'] = process.argv.slice(2);
  if (!['acquire', 'install'].includes(mode)) throw new Error('expected acquire or install');
  const temporary = mkdtempSync(join(tmpdir(), `oyzu-${profile.id}-`));
  const environment = {...process.env, HOME:join(temporary,'home'), USERPROFILE:join(temporary,'home'),
    XDG_CONFIG_HOME:join(temporary,'config'), XDG_DATA_HOME:join(temporary,'data'),
    npm_config_userconfig:join(temporary,'user.npmrc'), npm_config_globalconfig:join(temporary,'global.npmrc'),
    COREPACK_ENABLE_NETWORK:'0', YARN_IGNORE_PATH:'1'};
  mkdirSync(environment.HOME);
  const execute = args => {
    const result = spawnSync(profile.command[0], [...profile.command.slice(1), ...args],
      {cwd:workspace, env:environment, encoding:'utf8', timeout:240000, maxBuffer:8*1024*1024});
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`${profile.id} failed (${result.status}):\n${result.stdout}\n${result.stderr}`);
    return result.stdout.trim();
  };
  try {
    const version = execute(['--version']);
    const packageJson = JSON.parse(readFileSync(join(workspace,'package.json'),'utf8'));
    if (packageJson.packageManager != null && packageJson.packageManager !== `${profile.id}@${version}`) {
      throw new Error(`declared packageManager ${packageJson.packageManager} does not match provisioned ${profile.id}@${version}`);
    }
    const originalLock = readFileSync(join(workspace,profile.lock));
    const inventory = {version, nodeVersion:process.versions.node};
    if (mode === 'install') {
      const captured = JSON.parse(readFileSync(join(output,'inventory.json'),'utf8'));
      if (captured.version !== version || captured.nodeVersion !== inventory.nodeVersion) throw new Error('native manager differs from captured preflight');
    }
    execute(profile.install(mode, temporary));
    if (!readFileSync(join(workspace,profile.lock)).equals(originalLock)) throw new Error('native manager rewrote the frozen lockfile');
    if (mode === 'acquire') writeFileSync(join(output,'inventory.json'), JSON.stringify(inventory,null,2)+'\n');
  } finally { rmSync(temporary,{recursive:true,force:true}); }
}
