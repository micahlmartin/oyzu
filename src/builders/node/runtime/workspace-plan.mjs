// Private execution-copy projection. No source checkout or captured input is mutated.
import {createHash} from 'node:crypto';
import {lstatSync, mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import {join, resolve} from 'node:path';

export const root = process.cwd();
export const state = resolve('.oyzu-build');
export const digest = bytes => createHash('sha256').update(bytes).digest('hex');
export const read = file => JSON.parse(readFileSync(file, 'utf8'));

export function specification(encoded) {
  encoded ??= readFileSync(join(state, 'plan.json'), 'utf8');
  if (digest(encoded) !== process.env.OYZU_NODE_WORKSPACE_PLAN) throw new Error('workspace plan identity changed');
  return JSON.parse(encoded);
}

export function regular(file) {
  if (!lstatSync(file).isFile() || lstatSync(file).isSymbolicLink()) throw new Error(`Expected regular workspace output: ${file}`);
  return readFileSync(file);
}

export function begin(encoded) {
  const spec = specification(encoded);
  mkdirSync(state);
  writeFileSync(join(state,'plan.json'),encoded,{flag:'wx'});
  return spec;
}

export function projectDependencies(pkg, edges, versions) {
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
