// Native workspace membership and resolved local dependency edges, without scripts.
import {lstatSync, readFileSync, realpathSync} from 'node:fs';
import {isAbsolute, join, relative, resolve, sep} from 'node:path';
import {nativeRequire} from './npm-native.mjs';

export async function members(workspace, packageJson) {
  if (packageJson.workspaces == null) return [];
  const patterns = Array.isArray(packageJson.workspaces) ? packageJson.workspaces : packageJson.workspaces?.packages;
  if (!Array.isArray(patterns) || patterns.some(p => typeof p !== 'string' || !p ||
      p.includes('..') || /[\\:]/.test(p) || p.replace(/^!/, '').startsWith('/') || p.split('/').includes('node_modules'))) {
    throw new Error('npm workspace patterns must stay within captured source');
  }
  const root = realpathSync(workspace);
  const mapping = await nativeRequire('@npmcli/map-workspaces')({cwd: root, pkg: packageJson});
  if (mapping.size > 1024) throw new Error('npm workspace count exceeds limit');
  const result = [];
  const paths = new Set();
  for (const [name, directory] of mapping) {
    const path = relative(root, resolve(directory)).split(sep).join('/');
    if (!path || path.startsWith('../') || isAbsolute(path) || path.split('/').some(p => !p || p === '..') || paths.has(path.toLowerCase())) {
      throw new Error('Invalid or colliding native npm workspace path');
    }
    let candidate = root;
    for (const component of [...path.split('/'), 'package.json']) {
      candidate = join(candidate, component);
      if (lstatSync(candidate).isSymbolicLink()) throw new Error('npm workspace source cannot use symlinks');
    }
    const manifest = readFileSync(candidate);
    if (manifest.length > 4 * 1024 * 1024) throw new Error('npm workspace manifest exceeds limit');
    const pkg = JSON.parse(manifest);
    if (pkg.name !== name || !nativeRequire('semver').valid(pkg.version) || !/^(@[a-zA-Z0-9_.-]+\/)?[a-zA-Z0-9_.-]+$/.test(name)) {
      throw new Error('npm workspace requires a valid native name/version');
    }
    const scripts = pkg.scripts ?? {};
    if (!scripts || typeof scripts !== 'object' || Array.isArray(scripts) || Object.values(scripts).some(v => typeof v !== 'string')) {
      throw new Error('Invalid npm workspace scripts');
    }
    paths.add(path.toLowerCase());
    result.push({name, path, version: pkg.version, private: pkg.private === true, scripts});
  }
  return result.sort((a,b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
}

export async function graph(workspace, members) {
  if (!members.length) return null;
  const Arborist = nativeRequire('@npmcli/arborist');
  const tree = await new Arborist({path: workspace, offline: true, ignoreScripts: true}).loadActual();
  const paths = new Map(members.map(m => [realpathSync(join(workspace, m.path)), m.name]));
  const output = [];
  for (const member of members) {
    const link = tree.children.get(member.name);
    const node = link?.isLink ? link.target : link;
    if (!node || !node.isWorkspace || realpathSync(node.path) !== realpathSync(join(workspace, member.path))) {
      throw new Error(`Native npm installation did not link workspace ${member.name}`);
    }
    const dependencies = [];
    for (const edge of node.edgesOut.values()) {
      if (edge.error && !edge.optional) throw new Error(`Invalid native workspace dependency ${member.name}: ${edge.name}`);
      if (!edge.to) continue;
      const destination = edge.to.isLink ? edge.to.target : edge.to;
      const target = paths.get(realpathSync(destination.path));
      if (target) dependencies.push({name: edge.name, target, kind: edge.type, spec: edge.spec});
    }
    dependencies.sort((a,b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
    output.push({...member, dependencies});
  }
  return {schemaVersion: 1, members: output};
}
