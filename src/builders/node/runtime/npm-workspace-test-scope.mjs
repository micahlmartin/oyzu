// Package-owned test selection; explicit native scripts retain their own scope.
import {globSync, readFileSync, realpathSync} from 'node:fs';
import {createRequire} from 'node:module';
import {dirname, resolve} from 'node:path';

export function frameworkCommand(framework, root) {
  if (!['jest', 'vitest'].includes(framework)) throw new Error(`Unsupported native test entrypoint: ${framework}`);
  const native = createRequire(resolve(root, 'package.json'));
  const metadata = native.resolve(`${framework}/package.json`);
  const pkg = JSON.parse(readFileSync(metadata, 'utf8'));
  const bin = typeof pkg.bin === 'string' ? pkg.bin : pkg.bin?.[framework];
  if (typeof bin !== 'string' || !bin) throw new Error(`Missing native ${framework} bin declaration`);
  return [process.execPath, resolve(dirname(metadata), bin), framework === 'jest' ? '--ci' : 'run'];
}

export function nodeTests(root, modules) {
  // Node's documented default patterns; native glob excludes member trees
  // before traversal. Explicit scripts retain their own selection semantics.
  const extensions = process.features.typescript ? '{cjs,mjs,js,cts,mts,ts}' : '{cjs,mjs,js}';
  const patterns = ['**/*.test.', '**/*-test.', '**/*_test.', '**/test-*.', '**/test.', '**/test/**/*.'].map(p => p + extensions);
  const excluded = new Set(['node_modules', '.git', '.oyzu', '.oyzu-build', ...modules.map(m => m.path)]);
  const files = globSync(patterns, {cwd:root, exclude: path => {
    const normalized = path.replaceAll('\\', '/');
    return normalized.split('/').includes('node_modules') || [...excluded].some(p => normalized === p || normalized.startsWith(`${p}/`));
  }}).sort();
  if (!files.length) throw new Error('No package Node tests found outside workspace members');
  const absolute = files.map(path => resolve(root, path));
  if (absolute.length > 4096 || absolute.join('').length > 24_000) throw new Error('Package Node test selection exceeds argument limit');
  return absolute;
}

export function frameworkArguments(framework, root, modules, version) {
  const paths = ['.oyzu', '.oyzu-build', ...modules.map(m => m.path)];
  if (framework === 'vitest') return paths.map(path => `--exclude=${path}/**`);
  if (framework === 'jest') {
    const major = Number(version?.split('.')[0]);
    if (!Number.isSafeInteger(major) || major < 1) throw new Error('Missing native Jest version for workspace scope');
    const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    // An inclusion filter preserves the project's native ignore patterns.
    // Jest versions match either absolute or root-relative test paths.
    // Windows short names (RUNNER~1), junctions and POSIX symlink aliases can
    // differ from Jest's canonical absolute paths. Retain both spellings.
    const prefixes = [...new Set([root, realpathSync.native(root)].map(path => path.replaceAll('\\','/').replace(/^\/\/\?\/UNC\//, '//').replace(/^\/\/\?\//, '')))];
    const prefix = `(?:${prefixes.map(path => escape(`${path}/`)).join('|')})`;
    const excluded = paths.map(path => `(?:${prefix})?${escape(`${path}/`)}`).join('|');
    return [`--${major >= 30 ? 'testPathPatterns' : 'testPathPattern'}=^(?!(?:${excluded})).*`];
  }
  throw new Error(`Unsupported package framework scope: ${framework}`);
}
