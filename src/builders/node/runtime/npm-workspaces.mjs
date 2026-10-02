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
  if (packageJson.private !== true && (!nativeRequire('semver').valid(packageJson.version) ||
      typeof packageJson.name !== 'string' || !nativeRequire('validate-npm-package-name')(packageJson.name).validForNewPackages)) {
    throw new Error('Publishable npm workspace root requires a valid native name/version');
  }
  const root = realpathSync(workspace);
  const mapping = await nativeRequire('@npmcli/map-workspaces')({cwd: root, pkg: packageJson});
  if (mapping.size > 1024) throw new Error('npm workspace count exceeds limit');
  const result = [];
  const paths = new Set();
  for (const [name, directory] of mapping) {
    if (packageJson.private !== true && name === packageJson.name) throw new Error('Workspace root and member package names collide');
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
  // macOS /var -> /private/var (and other aliased checkout roots) must use
  // the same canonical root as native workspace membership discovery.
  workspace = realpathSync(workspace);
  const Arborist = nativeRequire('@npmcli/arborist');
  const tree = await new Arborist({path: workspace, offline: true, ignoreScripts: true}).loadActual();
  const paths = new Map(members.map(m => [realpathSync(join(workspace, m.path)), m.name]));
  const output = [];
  function edges(node) {
    const dependencies = [];
    for (const edge of node.edgesOut.values()) {
      // Arborist's synthetic membership edges contain absolute file: paths.
      // Membership is already captured above; only declared dependency edges
      // belong in the portable graph used for ordering/version projection.
      let kind = edge.type, spec = edge.spec;
      if (kind === 'workspace') {
        const declaration = [['optionalDependencies','optional'], ['dependencies','prod'], ['devDependencies','dev'], ['peerDependencies','peer']]
          .find(([field]) => Object.hasOwn(node.package[field] ?? {}, edge.name));
        if (!declaration) continue;
        kind = declaration[1];
        spec = node.package[declaration[0]][edge.name];
      }
      if (edge.error && !edge.optional) throw new Error(`Invalid native workspace dependency ${node.name}: ${edge.name}`);
      if (!edge.to) continue;
      const destination = edge.to.isLink ? edge.to.target : edge.to;
      const target = paths.get(realpathSync(destination.path));
      if (target) dependencies.push({name: edge.name, target, kind, spec});
    }
    return dependencies.sort((a,b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
  }
  for (const member of members) {
    const link = tree.children.get(member.name);
    const node = link?.isLink ? link.target : link;
    if (!node || !node.isWorkspace || realpathSync(node.path) !== realpathSync(join(workspace, member.path))) {
      throw new Error(`Native npm installation did not link workspace ${member.name}`);
    }
    output.push({...member, dependencies: edges(node)});
  }
  return {schemaVersion: 1, members: output, rootDependencies: edges(tree)};
}
