import {writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {begin, read, root, projectDependencies} from './workspace-plan.mjs';
const save = (file,value) => writeFileSync(file,JSON.stringify(value,null,2)+'\n');

export function project(encoded, dependencies = '/dependencies') {
  const spec = begin(encoded);
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
