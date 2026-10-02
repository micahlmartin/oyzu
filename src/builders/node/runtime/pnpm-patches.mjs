// Admit captured patch files; native pnpm owns selector matching, hashes and application.
import {existsSync, lstatSync, readFileSync} from 'node:fs';
import {join} from 'node:path';

function mapping(value, label) {
  if (value == null) return {};
  if (typeof value !== 'object' || Array.isArray(value)) throw new Error(`pnpm ${label} must be a mapping`);
  return value;
}

function patchFile(workspace, value) {
  if (typeof value !== 'string' || /[:\\\x00-\x1f\x7f]/.test(value)
      || value.split('/').some(part => !part || part === '..')) {
    throw new Error('pnpm patch path must be contained in the captured project');
  }
  let path = workspace;
  const parts = value.split('/');
  for (const [index, part] of parts.entries()) {
    path = join(path, part);
    const stat = lstatSync(path);
    if (stat.isSymbolicLink() || (index < parts.length - 1 ? !stat.isDirectory() : !stat.isFile())) {
      throw new Error('pnpm patch path must be a regular captured file without symbolic links');
    }
    if (index === parts.length - 1 && stat.size > 4*1024*1024) throw new Error('pnpm patch exceeds 4 MiB');
  }
}

export function validatePatches(workspace, manifest, lock, yaml) {
  const configs = [mapping(manifest.pnpm, 'package.json pnpm configuration')];
  const file = join(workspace, 'pnpm-workspace.yaml');
  if (existsSync(file)) configs.push(mapping(yaml.parse(readFileSync(file, 'utf8'),
    {maxAliasCount:100, uniqueKeys:true}), 'pnpm-workspace.yaml'));
  for (const config of configs) {
    for (const key of Object.keys(config)) {
      if (key !== 'patchedDependencies' && key !== 'packages') throw new Error(`pnpm capture does not yet support configuration ${key}`);
    }
    for (const path of Object.values(mapping(config.patchedDependencies, 'patchedDependencies'))) patchFile(workspace, path);
  }
  for (const value of Object.values(mapping(lock?.patchedDependencies, 'lock patchedDependencies'))) {
    patchFile(workspace, value?.path);
  }
}
