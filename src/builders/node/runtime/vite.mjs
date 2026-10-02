// Execute configured Vite through its native builder API. The manager still
// owns prebuild/postbuild; collection and tree identity remain engine-owned.
import {lstatSync, mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {isAbsolute, join, relative, resolve, sep} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath, pathToFileURL} from 'node:url';

const root = process.cwd();
const [operation, ...args] = process.argv.slice(2);
function readRecord(path, limit = 16384) {
  const info = lstatSync(path);
  if (!info.isFile() || info.isSymbolicLink() || info.size > limit) throw new Error('Invalid Vite metadata file');
  return JSON.parse(readFileSync(path, 'utf8'));
}
function contained(path, allowRoot = false) {
  const rel = relative(root, resolve(path)).split(sep).join('/');
  if (allowRoot && rel === '') return '.';
  if (!rel || isAbsolute(rel) || /[\\:\x00-\x1f]/.test(rel) || rel.split('/').some(p => !p || p === '.' || p === '..')) {
    throw new Error('Vite output/root must remain inside the captured target');
  }
  let current = root;
  for (const part of rel.split('/')) {
    current = join(current, part);
    const info = lstatSync(current, {throwIfNoEntry:false});
    if (info?.isSymbolicLink()) throw new Error('Vite output/root cannot use linked parents');
  }
  return rel;
}
if (operation === 'prepare') {
  const [record, expectedScript, outDir, ...extra] = args;
  const state = process.env.OYZU_NODE_VITE_STATE;
  if (!record || !expectedScript || outDir == null || extra.length || !state) throw new Error('Missing Vite preparation contract');
  const file = join(root, 'package.json');
  const pkg = readRecord(file, 4 * 1024 * 1024);
  if (pkg.scripts?.build !== expectedScript) throw new Error('Native install changed the planned Vite build script');
  const directory = join(root, '.oyzu-build');
  const info = lstatSync(directory, {throwIfNoEntry:false});
  if (info && (info.isSymbolicLink() || !info.isDirectory())) throw new Error('Unsafe Vite wrapper directory');
  mkdirSync(directory, {recursive:true});
  writeFileSync(join(directory, 'vite-entry.mjs'), `await import(${JSON.stringify(import.meta.url)});\n`, {flag:'wx'});
  writeFileSync(state, JSON.stringify({record, outDir:outDir || null}), {flag:'wx'});
  pkg.scripts.build = 'node .oyzu-build/vite-entry.mjs build';
  writeFileSync(file, JSON.stringify(pkg, null, 2)+'\n');
} else if (operation === 'build') {
  if (args.length) throw new Error('Configured Vite wrapper does not accept forwarded arguments');
  const state = readRecord(process.env.OYZU_NODE_VITE_STATE);
  if (typeof state.record !== 'string' || (state.outDir !== null && typeof state.outDir !== 'string') || !process.env.OYZU_VERSION) throw new Error('Invalid Vite preparation state');
  const project = createRequire(join(root, 'package.json'));
  const version = project('vite/package.json').version;
  if (!version.startsWith('8.')) throw new Error(`Configured Vite ${version} needs a supported native API adapter (currently Vite 8)`);
  const {createBuilder} = await import(pathToFileURL(project.resolve('vite')).href);
  const written = new Set();
  let expectedOutput;
  function outputPath(options) {
    if (!options.dir || options.file) throw new Error('Vite application requires directory output');
    const directory = contained(options.dir);
    if (directory !== expectedOutput) throw new Error('Native Vite outputs differ from the resolved application directory');
    return directory;
  }
  const observer = {name:'oyzu-output-evidence', apply:'build', enforce:'post',
    outputOptions:{order:'post', handler(options) { outputPath(options); }},
    writeBundle:{order:'post', handler(options) {
      written.add(outputPath(options));
    }},
  };
  const builder = await createBuilder({
    ...(state.outDir ? {build:{outDir:state.outDir}} : {}), plugins:[observer],
  }, null);
  const environments = Object.entries(builder.environments);
  if (environments.length !== 1 || environments[0][0] !== 'client') throw new Error('Multiple/SSR Vite environments require separate artifact declarations');
  const environment = environments[0][1];
  const config = environment.config;
  const nativeRoot = contained(config.root, true);
  const output = contained(resolve(config.root, config.build.outDir));
  expectedOutput = output;
  if (config.build.watch || config.build.write === false || config.build.ssr || config.build.lib) throw new Error('Vite application requires a finite client build with written output');
  await builder.buildApp();
  if (!environment.isBuilt || written.size !== 1 || !written.has(output)) throw new Error('Native Vite outputs differ from the resolved application directory');
  const record = {schemaVersion:'v1alpha1', kind:'vite-output', toolVersion:version,
    version:process.env.OYZU_VERSION, root:nativeRoot, output, mode:config.mode};
  const bytes = JSON.stringify(record, null, 2)+'\n';
  if (Buffer.byteLength(bytes) > 16384) throw new Error('Vite output metadata exceeds limit');
  writeFileSync(state.record, bytes, {flag:'wx'});
} else if (operation === 'package') {
  const [path, destination, ...extra] = args;
  if (!path || !destination || extra.length) throw new Error('Missing Vite package contract');
  const record = readRecord(path);
  if (record.schemaVersion !== 'v1alpha1' || record.kind !== 'vite-output' || record.version !== process.env.OYZU_VERSION
      || typeof record.toolVersion !== 'string' || !record.toolVersion.startsWith('8.') || typeof record.mode !== 'string'
      || typeof record.root !== 'string' || contained(record.root, true) !== record.root
      || typeof record.output !== 'string' || contained(record.output) !== record.output) throw new Error('Invalid native Vite output evidence');
  const runtime = fileURLToPath(new URL('./node-application.mjs', import.meta.url));
  const result = spawnSync(process.execPath, [runtime, record.output, destination], {stdio:'inherit'});
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
} else {
  throw new Error('Unknown Vite integration operation');
}
