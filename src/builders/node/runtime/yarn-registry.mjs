// Yarn Classic lock interpretation and native offline-mirror installation.
import {createRequire} from 'node:module';
import {copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {isDeepStrictEqual} from 'node:util';
import {archives} from './registry-archives.mjs';

const require = createRequire(import.meta.url);
function parse(text) {
  const parser = require(process.env.OYZU_YARN_LOCKFILE ?? '/opt/oyzu-yarn/node_modules/@yarnpkg/lockfile');
  const lock = parser.parse(text);
  if (lock.type !== 'success') throw new Error('Yarn lock must be fully resolved without merge conflicts');
  return lock.object;
}
export function inputs(workspace) {
  for (const name of ['.yarnrc', '.yarnrc.yml', '.npmrc']) {
    if (existsSync(join(workspace, name))) throw new Error(`Yarn capture does not yet support ${name}`);
  }
  const manifest = JSON.parse(readFileSync(join(workspace, 'package.json'), 'utf8'));
  if (manifest.workspaces) throw new Error('Yarn workspace capture is not implemented yet');
  if (manifest.resolutions != null) {
    if (typeof manifest.resolutions !== 'object' || Array.isArray(manifest.resolutions)
        || Object.values(manifest.resolutions).some(value => typeof value !== 'string' || /[:/\\]/.test(value))) {
      throw new Error('Yarn resolutions must use registry version ranges; file, Git and URL overrides are not supported');
    }
  }
  const text = readFileSync(join(workspace, 'yarn.lock'), 'utf8');
  if (!text.split(/\r?\n/).some(line => line.trim() === '# yarn lockfile v1')) throw new Error('Yarn capture requires a Classic v1 lock');
  const lock = parse(text);
  for (const field of ['dependencies', 'devDependencies', 'optionalDependencies']) {
    for (const [name, version] of Object.entries(manifest[field] ?? {})) {
      if (!lock[`${name}@${version}`]) throw new Error(`Yarn capture requires a current frozen lockfile for ${name}`);
    }
  }
  const packages = new Map();
  for (const [selector, value] of Object.entries(lock)) {
    const url = new URL(value.resolved);
    if (url.protocol !== 'https:' || url.username || url.password || url.search || (url.hash && !/^#[a-f0-9]{40}$/.test(url.hash))) {
      throw new Error(`Yarn capture requires a credential-free registry archive: ${selector}`);
    }
    url.hash = '';
    const separator = url.pathname.indexOf('/-/');
    const name = url.pathname.slice(1, separator);
    const version = value.version;
    if (separator < 0 || !/^(?:@[a-zA-Z0-9_.-]+\/)?[a-zA-Z0-9_.-]+$/.test(name)
        || typeof version !== 'string' || !/^[0-9]+\.[0-9]+\.[0-9]+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?$/.test(version)
        || url.pathname.slice(separator+3) !== `${name.split('/').at(-1)}-${version}.tgz`) {
      throw new Error(`unsupported Yarn registry archive: ${selector}`);
    }
    if (!/^sha512-[A-Za-z0-9+/]{86}==$/.test(value.integrity)) throw new Error(`Yarn package requires sha512 integrity: ${selector}`);
    const id = `${name}@${version}`;
    const entry = {id, name, version, url:url.href, integrity:value.integrity,
      mirror:`${name.replace('/', '-')}-${version}.tgz`};
    const previous = packages.get(id);
    if (previous && (previous.url !== entry.url || previous.integrity !== entry.integrity)) throw new Error(`conflicting Yarn archive identity: ${id}`);
    packages.set(id, entry);
  }
  if (packages.size > 4096) throw new Error('Yarn capture exceeds package limit');
  const ordered = [...packages.values()].sort((a,b) => a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
  const mirrors = new Set();
  for (const entry of ordered) {
    if (mirrors.has(entry.mirror)) throw new Error(`ambiguous Yarn mirror filename: ${entry.mirror}`);
    mirrors.add(entry.mirror);
  }
  return ordered;
}

export async function perform(context) {
  const {mode, output, workspace, temporary, execute, captured} = context;
  const entries = inputs(workspace);
  if (mode === 'install' && captured.layoutVersion !== 2) throw new Error('Yarn capture layout changed; reacquire dependencies');
  const packages = await archives(context, entries);
  const mirror = join(temporary, 'mirror');
  mkdirSync(mirror);
  for (const entry of packages) copyFileSync(join(output, 'tarballs', `${entry.sha256}.tgz`), join(mirror, entry.mirror));
  const config = join(temporary, 'yarnrc');
  writeFileSync(config, `yarn-offline-mirror ${JSON.stringify(mirror)}\nyarn-offline-mirror-pruning false\ndisable-self-update-check true\n`);
  const lockPath = join(workspace, 'yarn.lock');
  const originalLock = readFileSync(lockPath);
  const install = ['install', '--offline', '--non-interactive', '--production=false',
    '--use-yarnrc', config, '--cache-folder', join(temporary, 'cache')];
  // Yarn's frozen mode can suppress lock writes for a changed resolution. Let
  // native Yarn compute its actual lock in this private tree with hooks disabled,
  // then compare parsed identities before allowing any lifecycle execution.
  await execute([...install, '--ignore-scripts', '--force']);
  if (!isDeepStrictEqual(parse(originalLock.toString('utf8')), parse(readFileSync(lockPath, 'utf8')))) {
    throw new Error('Yarn requires a current frozen lockfile; native dependency resolution changed');
  }
  // Native formatting is not dependency identity; preserve the exact source lock.
  writeFileSync(lockPath, originalLock);
  if (mode === 'install') await execute([...install, '--frozen-lockfile', '--force']);
  return {layoutVersion:2, packages};
}

// Only already-admitted and verified archives enter the portable offline mirror.
// Native cache metadata, temporary paths, configuration and logs stay private.
export function exportStore({output, inventory}) {
  const mirror = join(output, 'mirror');
  mkdirSync(mirror);
  for (const entry of inventory.packages) {
    copyFileSync(join(output, 'tarballs', `${entry.sha256}.tgz`), join(mirror, entry.mirror));
  }
}
