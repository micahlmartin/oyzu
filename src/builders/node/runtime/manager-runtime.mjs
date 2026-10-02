// Shared lifecycle only. Each native manager supplies its own command/arguments.
import {spawn} from 'node:child_process';
import {mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {verifyNodeVersion} from './node-runtime.mjs';

export async function run(profile) {
  const [operation, output = operation === 'install' ? '/dependencies' : '/out', workspace = '/workspace', broker = '/broker'] = process.argv.slice(2);
  if (!['acquire', 'acquire-context', 'install'].includes(operation)) throw new Error('expected acquire, acquire-context or install');
  if (operation === 'acquire-context' && !profile.exportStore) throw new Error('native manager has no dependency-context store export');
  const mode = operation === 'acquire-context' ? 'acquire' : operation;
  verifyNodeVersion();
  const temporary = mkdtempSync(join(tmpdir(), `oyzu-${profile.id}-`));
  const environment = {...process.env, HOME:join(temporary,'home'), USERPROFILE:join(temporary,'home'),
    XDG_CONFIG_HOME:join(temporary,'config'), XDG_DATA_HOME:join(temporary,'data'),
    npm_config_userconfig:join(temporary,'user.npmrc'), npm_config_globalconfig:join(temporary,'global.npmrc'),
    COREPACK_ENABLE_NETWORK:'0', YARN_IGNORE_PATH:'1'};
  mkdirSync(environment.HOME);
  const execute = args => new Promise((resolve, reject) => {
    const child = spawn(profile.command[0], [...profile.command.slice(1), ...args],
      {cwd:workspace, env:environment, stdio:['ignore','pipe','pipe'], timeout:240000});
    let stdout = '', stderr = '', bytes = 0, exceeded = false;
    const collect = (chunk, error) => {
      bytes += chunk.length;
      if (bytes > 8*1024*1024) { exceeded = true; child.kill(); }
      else if (error) stderr += chunk.toString('utf8');
      else stdout += chunk.toString('utf8');
    };
    child.stdout.on('data', chunk => collect(chunk, false)); child.stderr.on('data', chunk => collect(chunk, true));
    child.on('error', reject);
    child.on('close', (code, signal) => {
      if (exceeded || code !== 0 || signal) reject(new Error(`${profile.id} failed (${signal ?? code}${exceeded ? ', output limit' : ''}):\n${stdout}\n${stderr}`));
      else resolve(stdout.trim());
    });
  });
  try {
    if (profile.validate) profile.validate(workspace);
    const version = await execute(['--version']);
    const packageJson = JSON.parse(readFileSync(join(workspace,'package.json'),'utf8'));
    if (packageJson.packageManager != null && packageJson.packageManager !== `${profile.id}@${version}`) {
      throw new Error(`declared packageManager ${packageJson.packageManager} does not match provisioned ${profile.id}@${version}`);
    }
    const originalLock = readFileSync(join(workspace,profile.lock));
    const inventory = {version, nodeVersion:process.versions.node};
    let captured;
    if (mode === 'install') {
      captured = JSON.parse(readFileSync(join(output,'inventory.json'),'utf8'));
      if (captured.version !== version || captured.nodeVersion !== inventory.nodeVersion) throw new Error('native manager differs from captured preflight');
    }
    if (profile.perform) Object.assign(inventory, await profile.perform({mode, output, workspace, broker, temporary, environment, execute, captured}));
    else await execute(profile.install(mode, temporary));
    if (!readFileSync(join(workspace,profile.lock)).equals(originalLock)) throw new Error('native manager rewrote the frozen lockfile');
    // Store export happens only after native validation succeeds. Managers own
    // layout; the lifecycle never exposes its temporary cache/configuration.
    if (operation === 'acquire-context') await profile.exportStore(output, inventory);
    if (mode === 'acquire') writeFileSync(join(output,'inventory.json'), JSON.stringify(inventory,null,2)+'\n');
  } finally { rmSync(temporary,{recursive:true,force:true}); }
}
