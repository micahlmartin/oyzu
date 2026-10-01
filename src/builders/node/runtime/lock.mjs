// Interpret only immutable npm registry tarball inputs. Native npm owns resolution,
// peer relationships, installation layout and lock/manifest consistency checking.
import {createHash, timingSafeEqual} from 'node:crypto';
import {existsSync, readFileSync} from 'node:fs';
import {join} from 'node:path';

export function readLock(workspace) {
  const filename = existsSync(join(workspace, 'npm-shrinkwrap.json'))
    ? 'npm-shrinkwrap.json' : 'package-lock.json';
  if (!existsSync(join(workspace, filename))) {
    const packageJson = JSON.parse(readFileSync(join(workspace, 'package.json'), 'utf8'));
    if (['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies']
      .some(field => Object.keys(packageJson[field] ?? {}).length > 0)) {
      throw new Error('npm dependencies require a captured lockfile');
    }
    return {filename: null, packages: []};
  }
  const lock = JSON.parse(readFileSync(join(workspace, filename), 'utf8'));
  if (![2, 3].includes(lock.lockfileVersion) || !lock.packages || Array.isArray(lock.packages)) {
    throw new Error('npm acquisition requires a v2/v3 lockfile with packages');
  }
  const entries = Object.entries(lock.packages).filter(([path]) => path !== '').sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0);
  if (entries.length > 4096) throw new Error('npm lock exceeds acquisition package limit');
  const packages = entries.map(([path, entry]) => {
    if (!/^(?:node_modules\/(?:@[a-zA-Z0-9_.-]+\/)?[a-zA-Z0-9_.-]+\/)*node_modules\/(?:@[a-zA-Z0-9_.-]+\/)?[a-zA-Z0-9_.-]+$/.test(path)
        || path.split('/').some(p => p === '.' || p === '..') || !entry || typeof entry !== 'object'
        || entry.link || entry.inBundle || !entry.version || typeof entry.version !== 'string') {
      throw new Error(`unsupported npm lock package ${path}`);
    }
    const url = new URL(entry.resolved);
    if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash) {
      throw new Error(`npm package ${path} requires a credential-free HTTPS tarball`);
    }
    // A narrow admitted profile: sha512 is required; weaker or mixed SRI forms
    // need an explicit adapter extension, not a silent downgrade.
    if (typeof entry.integrity !== 'string' || !/^sha512-[A-Za-z0-9+/]{86}==$/.test(entry.integrity)) {
      throw new Error(`npm package ${path} requires a sha512 integrity value`);
    }
    return {path, name: entry.name ?? path.split('node_modules/').at(-1),
      version: entry.version, url: url.href, integrity: entry.integrity,
      purpose: entry.dev ? 'build' : 'runtime'};
  });
  return {filename, packages};
}

export function verify(bytes, integrity) {
  const expected = Buffer.from(integrity.slice('sha512-'.length), 'base64');
  const actual = createHash('sha512').update(bytes).digest();
  if (expected.length !== actual.length || !timingSafeEqual(expected, actual)) {
    throw new Error('npm tarball integrity mismatch');
  }
  return createHash('sha256').update(bytes).digest('hex');
}
