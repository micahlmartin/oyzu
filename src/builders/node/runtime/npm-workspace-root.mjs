// Native root packaging within an npm workspace.
import {chmodSync, copyFileSync, lstatSync, mkdirSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {npm} from './npm-native.mjs';

function parts(path) {
  if (typeof path !== 'string' || /[\\:\x00-\x1f]/.test(path) || path.split('/').some(p => !p || p === '.' || p === '..')) {
    throw new Error('Invalid native npm package file path');
  }
  return path.split('/');
}

export function packRoot(root, destination, cache) {
  const flags = ['pack', '--ignore-scripts', '--json', '--workspaces=false'];
  const selected = JSON.parse(npm([...flags, '--dry-run'], root, cache));
  if (selected.length !== 1 || !Array.isArray(selected[0].files) || selected[0].files.length > 100_000) {
    throw new Error('Invalid native npm root pack inventory');
  }
  const stage = join(cache, 'root-package');
  mkdirSync(stage);
  const expected = new Set();
  let bytes = 0;
  for (const entry of selected[0].files) {
    const components = parts(entry.path);
    if (['.oyzu', '.oyzu-build'].includes(components[0])) continue;
    let source = root;
    for (const component of components) {
      source = join(source, component);
      if (lstatSync(source).isSymbolicLink()) throw new Error('npm root package source cannot use symlinks');
    }
    const stat = lstatSync(source);
    if (!stat.isFile() || stat.size !== entry.size || expected.has(entry.path)) throw new Error('Invalid native npm root package file');
    bytes += stat.size;
    if (bytes > 1024 * 1024 * 1024) throw new Error('npm root package exceeds staging limit');
    expected.add(entry.path);
    const target = join(stage, ...components);
    mkdirSync(dirname(target), {recursive:true});
    copyFileSync(source, target);
    chmodSync(target, stat.mode & 0o777);
  }
  // npm owns ignore/files/bundling semantics and tar creation. The private
  // staging tree removes engine-owned state without rewriting package.json.
  const packed = JSON.parse(npm([...flags, '--pack-destination', destination], stage, cache));
  if (packed.length !== 1 || packed[0].files.length !== expected.size ||
      packed[0].files.some(file => !expected.delete(file.path)) || expected.size) {
    throw new Error('Native npm root pack file selection changed during staging');
  }
  return packed;
}
