// pnpm owns lock/manifest consistency, peers and installation layout. This adapter
// captures locked registry archives and serves only those bytes over loopback.
import {createRequire} from 'node:module';
import {createServer} from 'node:http';
import {existsSync, readFileSync} from 'node:fs';
import {join} from 'node:path';
import {archives} from './registry-archives.mjs';
import {validatePatches} from './pnpm-patches.mjs';

const require = createRequire(import.meta.url);
export function inputs(workspace) {
  const yaml = require(process.env.OYZU_PNPM_YAML ?? '/opt/oyzu-pnpm/node_modules/yaml');
  const lock = yaml.parse(readFileSync(join(workspace, 'pnpm-lock.yaml'), 'utf8'), {maxAliasCount: 100, uniqueKeys: true});
  const manifest = JSON.parse(readFileSync(join(workspace, 'package.json'), 'utf8'));
  for (const name of ['.npmrc', '.pnpmfile.cjs']) {
    if (existsSync(join(workspace, name))) throw new Error(`pnpm capture does not yet support ${name}`);
  }
  if (manifest.workspaces) throw new Error('pnpm workspace capture is not implemented yet');
  validatePatches(workspace, manifest, lock, yaml);
  if (String(lock?.lockfileVersion) !== '9.0' || Object.keys(lock.importers ?? {}).join() !== '.') throw new Error('pnpm capture requires a single-project v9 lockfile');
  for (const key of ['overrides', 'packageExtensionsChecksum']) {
    if (lock[key] != null) throw new Error(`pnpm capture does not yet support ${key}`);
  }
  for (const owner of [lock.importers['.'], ...Object.values(lock.snapshots ?? {})]) {
    for (const field of ['dependencies', 'devDependencies', 'optionalDependencies']) {
      for (const value of Object.values(owner[field] ?? {})) {
        const version = typeof value === 'string' ? value : value?.version;
        if (typeof version !== 'string' || /^(?:file:|link:|workspace:|https?:|git(?:\+|:|@)|github:|\/|\.{1,2}\/)/.test(version)) {
          throw new Error('pnpm capture does not yet support non-registry dependency references');
        }
      }
    }
  }
  const packages = Object.entries(lock.packages ?? {}).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0);
  if (packages.length > 4096) throw new Error('pnpm capture exceeds package limit');
  return packages.map(([id, value]) => {
    const match = /^((?:@[a-zA-Z0-9_.-]+\/)?[a-zA-Z0-9_.-]+)@([0-9]+\.[0-9]+\.[0-9]+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?)$/.exec(id);
    if (!match || Object.keys(value.resolution ?? {}).join() !== 'integrity') throw new Error(`pnpm capture requires an integrity-locked registry package: ${id}`);
    const [, name, version] = match;
    const integrity = value.resolution.integrity;
    if (!/^sha512-[A-Za-z0-9+/]{86}==$/.test(integrity)) throw new Error(`pnpm package requires sha512 integrity: ${id}`);
    const path = `/${name}/-/${name.split('/').at(-1)}-${version}.tgz`;
    if (new URL(`https://registry.npmjs.org${path}`).pathname !== path) throw new Error(`invalid pnpm package path: ${id}`);
    return {id, name, version, integrity, path, url: `https://registry.npmjs.org${path}`};
  });
}

export async function perform({mode, output, workspace, broker, temporary, environment, execute, captured}) {
  const entries = inputs(workspace);
  if (mode === 'install' && captured.layoutVersion !== 2) throw new Error('pnpm capture layout changed; reacquire dependencies');
  const packages = await archives({mode, output, broker, captured}, entries);
  const served = new Map();
  for (const entry of packages) {
    // Keep archive bytes on disk, rather than retaining the entire graph in RAM.
    served.set(entry.path, join(output, 'tarballs', `${entry.sha256}.tgz`));
  }
  const server = createServer((request, response) => {
    const file = served.get(request.url);
    if (request.method !== 'GET' || !file) { response.writeHead(404); response.end(); return; }
    try {
      const body = readFileSync(file);
      response.writeHead(200, {'Content-Type':'application/octet-stream', 'Content-Length':body.length});
      response.end(body);
    } catch { response.writeHead(500); response.end(); }
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  try {
    // No forwarding occurs here, even during lifecycle scripts. Acquisition has
    // finished; this server serves an exact archive allowlist in a networkless worker.
    environment.CI = 'true';
    await execute(['install', '--frozen-lockfile', '--ignore-pnpmfile',
      `--ignore-scripts=${mode === 'acquire'}`, '--config.engine-strict=true', '--config.force=false',
      '--config.offline=false', '--config.production=false', '--config.side-effects-cache=false',
      '--config.fetch-retries=0', '--config.fetch-timeout=15000',
      `--registry=http://127.0.0.1:${server.address().port}/`, '--store-dir', join(temporary, 'store')]);
  } finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
  return {layoutVersion:2, packages};
}
