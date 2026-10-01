import {spawnSync} from 'node:child_process';
import {mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join} from 'node:path';
import {fetch} from './broker_transport.mjs';
import {readLock, verify} from './npm_lock.mjs';

function npm(args, workspace, cache) {
  // npm.cmd cannot be spawned directly on Windows. Invoke its installed JS entry
  // point with Node there, without a shell or interpolated command string.
  const command = process.platform === 'win32' ? process.execPath : 'npm';
  const prefix = process.platform === 'win32'
    ? [join(dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js')] : [];
  const result = spawnSync(command, [...prefix, ...args, '--offline', '--audit=false', '--fund=false',
    '--update-notifier=false', '--engine-strict=true', '--force=false', '--cache', cache, '--userconfig', join(cache, 'user.npmrc'),
    '--globalconfig', join(cache, 'global.npmrc')], {
    cwd: workspace, encoding: 'utf8', timeout: 240_000, maxBuffer: 8 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`npm ${args[0]} failed (${result.status}):\n${result.stdout}\n${result.stderr}`);
  return result.stdout;
}

const [mode, root = mode === 'acquire' ? '/out' : '/dependencies', workspace = '/workspace', broker = '/broker'] = process.argv.slice(2);
if (!['acquire', 'install'].includes(mode)) throw new Error('expected npm acquire or install');
const lock = readLock(workspace);
const cache = mkdtempSync(join(tmpdir(), 'oyzu-npm-'));
try {
  const version = npm(['--version'], workspace, cache).trim();
  const packageJson = JSON.parse(readFileSync(join(workspace, 'package.json'), 'utf8'));
  if (packageJson.packageManager != null && packageJson.packageManager !== `npm@${version}`) {
    throw new Error(`declared packageManager ${packageJson.packageManager} does not match provisioned npm@${version}; select a matching provisioned toolchain image`);
  }
  const packages = [];
  const loaded = new Set();
  const inventory = mode === 'install' ? JSON.parse(readFileSync(join(root, 'inventory.json'), 'utf8')) : null;
  if (inventory && (inventory.version !== version || inventory.nodeVersion !== process.versions.node)) {
    throw new Error('Node/npm runtime differs from the captured preflight toolchain');
  }
  if (mode === 'acquire') mkdirSync(join(root, 'tarballs'), {recursive: true});
  for (const entry of lock.packages) {
    let body, sourceId;
    if (mode === 'acquire') {
      const response = await fetch(entry.url, broker);
      if (response.info.status !== 200) throw new Error(`npm acquisition denied or failed for ${entry.path}`);
      ({body} = response);
      sourceId = response.info.sourceId;
    } else {
      const stored = inventory.packages.find(p => p.path === entry.path && p.integrity === entry.integrity && p.url === entry.url);
      if (!stored || !/^[a-f0-9]{64}$/.test(stored.sha256)) throw new Error(`missing captured npm package ${entry.path}`);
      body = readFileSync(join(root, 'tarballs', `${stored.sha256}.tgz`));
    }
    const sha256 = verify(body, entry.integrity);
    const tarball = join(root, 'tarballs', `${sha256}.tgz`);
    if (mode === 'acquire' && !loaded.has(sha256)) writeFileSync(tarball, body, {flag: 'wx'});
    if (!loaded.has(sha256)) npm(['cache', 'add', tarball, '--ignore-scripts'], workspace, cache);
    loaded.add(sha256);
    packages.push({...entry, sha256, size: body.length, sourceId});
  }
  // Native npm rejects stale lockfiles and validates target installation. Never
  // execute lifecycle code while the broker is mounted. Execution has no broker.
  const install = lock.filename ? ['ci'] : ['install', '--package-lock=false'];
  npm([...install, `--ignore-scripts=${mode === 'acquire'}`, '--include=dev', '--include=optional', '--include=peer'], workspace, cache);
  if (mode === 'acquire') {
    writeFileSync(join(root, 'inventory.json'), JSON.stringify({version, nodeVersion: process.versions.node, lockfile: lock.filename, packages}, null, 2) + '\n');
  }
} finally {
  rmSync(cache, {recursive: true, force: true});
}
