// Native root packaging and implicit test scope within an npm workspace.
import {chmodSync, copyFileSync, globSync, lstatSync, mkdirSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
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

export function rootNodeTests(root, modules) {
  // Node's documented default patterns; native glob excludes member trees
  // before traversal. Explicit scripts retain their own selection semantics.
  const extensions = process.features.typescript ? '{cjs,mjs,js,cts,mts,ts}' : '{cjs,mjs,js}';
  const patterns = ['**/*.test.', '**/*-test.', '**/*_test.', '**/test-*.', '**/test.', '**/test/**/*.'].map(p => p + extensions);
  const excluded = new Set(['node_modules', '.git', '.oyzu', '.oyzu-build', ...modules.map(m => m.path)]);
  const files = globSync(patterns, {cwd:root, exclude: path => {
    const normalized = path.replaceAll('\\', '/');
    return normalized.split('/').includes('node_modules') || [...excluded].some(p => normalized === p || normalized.startsWith(`${p}/`));
  }}).sort();
  if (!files.length) throw new Error('No root Node tests found outside workspace members');
  const absolute = files.map(path => resolve(root, path));
  if (absolute.length > 4096 || absolute.join('').length > 24_000) throw new Error('Root Node test selection exceeds argument limit');
  return absolute;
}

export function rootFrameworkArguments(framework, root, modules, version) {
  const paths = ['.oyzu', '.oyzu-build', ...modules.map(m => m.path)];
  if (framework === 'vitest') return paths.map(path => `--exclude=${path}/**`);
  if (framework === 'jest') {
    const major = Number(version?.split('.')[0]);
    if (!Number.isSafeInteger(major) || major < 1) throw new Error('Missing native Jest version for workspace scope');
    const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    // An inclusion filter preserves the project's native ignore patterns.
    // Jest versions match either absolute or root-relative test paths.
    const prefix = escape(`${root.replaceAll('\\','/')}/`);
    const excluded = paths.map(path => `(?:${prefix})?${escape(`${path}/`)}`).join('|');
    return [`--${major >= 30 ? 'testPathPatterns' : 'testPathPattern'}=^(?!(?:${excluded})).*`];
  }
  throw new Error(`Unsupported root framework scope: ${framework}`);
}
