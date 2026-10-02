// Private execution-copy projection. No source checkout or captured input is mutated.
import {createHash} from 'node:crypto';
import {lstatSync, mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import {join, resolve} from 'node:path';

export const root = process.cwd();
export const state = resolve('.oyzu-build');
export const digest = bytes => createHash('sha256').update(bytes).digest('hex');
export const read = file => JSON.parse(readFileSync(file, 'utf8'));
const save = (file, value) => writeFileSync(file, JSON.stringify(value, null, 2) + '\n');

export function specification(encoded) {
  encoded ??= readFileSync(join(state, 'plan.json'), 'utf8');
  if (digest(encoded) !== process.env.OYZU_NPM_WORKSPACE_PLAN) throw new Error('npm workspace plan identity changed');
  return JSON.parse(encoded);
}

function projectDependencies(pkg, edges, versions) {
  const fields = {prod:'dependencies', dev:'devDependencies', optional:'optionalDependencies', peer:'peerDependencies', peerOptional:'peerDependencies'};
  for (const edge of edges) {
    const field = fields[edge.kind];
    if (!field || pkg[field]?.[edge.name] !== edge.spec || !versions.has(edge.target)) {
      throw new Error(`Workspace dependency no longer matches captured graph: ${edge.name}`);
    }
    // npm permits a dependency to appear in several declaration fields.
    for (const candidate of Object.values(fields)) {
      if (Object.hasOwn(pkg[candidate] ?? {}, edge.name)) pkg[candidate][edge.name] = versions.get(edge.target);
    }
  }
}

export function project(encoded, dependencies = '/dependencies') {
  const spec = specification(encoded);
  mkdirSync(state); // reject source collisions and repeated preparation
  writeFileSync(join(state, 'plan.json'), encoded, {flag:'wx'});
  const inventory = read(join(dependencies, 'inventory.json'));
  if (!['package-lock.json', 'npm-shrinkwrap.json'].includes(inventory.lockfile)) throw new Error('Workspace requires captured native lockfile');
  const lockPath = join(root, inventory.lockfile);
  const lock = read(lockPath);
  const versions = new Map(spec.modules.map(m => [m.name, m.version]));
  const entries = [{path:'.', version:spec.rootVersion, dependencies:spec.rootDependencies}, ...spec.modules];
  for (const entry of entries) {
    const path = join(root, entry.path, 'package.json');
    const pkg = read(path);
    if (entry.name && pkg.name !== entry.name) throw new Error('Workspace package identity changed');
    projectDependencies(pkg, entry.dependencies, versions);
    pkg.version = entry.version;
    const record = lock.packages[entry.path === '.' ? '' : entry.path];
    if (!record) throw new Error('Workspace lock record disappeared');
    record.version = pkg.version;
    for (const field of ['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies']) {
      if (pkg[field]) record[field] = pkg[field];
    }
    save(path, pkg);
  }
  lock.version = spec.rootVersion;
  save(lockPath, lock);
}

export function regular(file) {
  if (!lstatSync(file).isFile() || lstatSync(file).isSymbolicLink()) throw new Error(`Expected regular workspace output: ${file}`);
  return readFileSync(file);
}
