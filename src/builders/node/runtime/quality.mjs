// Native quality APIs; usable as a runtime file or embedded development command.
import {existsSync, lstatSync, readFileSync, readdirSync, writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {dirname, isAbsolute, join, relative, resolve, sep} from 'node:path';
import {pathToFileURL} from 'node:url';

const root = process.cwd();
const args = process.argv.slice(process.execArgv.includes('-e') ? 1 : 2);
if (args.length !== 1) throw new Error('Implicit quality checks do not accept extra arguments; define a native script or an argv task');
const biome = args[0].startsWith('biome-');
const mode = args[0].replace(/^biome-/, '');
if (!['lint', 'format-check', 'format'].includes(mode)) throw new Error('Expected Node quality operation');
const project = createRequire(join(root, 'package.json'));
const fallback = process.env.OYZU_NODE_QUALITY_HOME
  ? createRequire(join(resolve(process.env.OYZU_NODE_QUALITY_HOME), 'package.json')) : null;
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
function locate(name, request = name) {
  let resolved;
  try { resolved = project.resolve(request); }
  catch (error) {
    if (error.code !== 'MODULE_NOT_FOUND') throw error;
    if (['dependencies','devDependencies','optionalDependencies'].some(k => Object.hasOwn(pkg[k] ?? {}, name))) {
      throw new Error(`Declared ${name} is not installed; prepare the project's native dependencies first`);
    }
    if (!fallback) throw new Error(`No provisioned ${name}; supply project dependencies or OYZU_NODE_QUALITY_HOME`);
    resolved = fallback.resolve(request);
  }
  return resolved;
}
const library = name => project(locate(name));
const excluded = new Set(['node_modules','.git','.oyzu','.oyzu-build','dist','build','coverage']);
const scopes = JSON.parse(process.env.OYZU_NODE_QUALITY_EXCLUDE ?? '[]');
if (process.env.OYZU_NODE_VITE_CONFIG === '1') {
  function nativeRecord(path) {
    const info = lstatSync(path);
    if (!info.isFile() || info.isSymbolicLink() || info.size > 16384) throw new Error('Invalid Vite output metadata');
    return JSON.parse(readFileSync(path, 'utf8'));
  }
  const recordPath = process.env.OYZU_NODE_VITE_RECORD;
  const info = recordPath && lstatSync(recordPath, {throwIfNoEntry:false});
  if (info) {
    const record = nativeRecord(recordPath);
    if (record.schemaVersion !== 'v1alpha1' || record.kind !== 'vite-output' || record.version !== process.env.OYZU_VERSION) throw new Error('Mismatched Vite output metadata');
    scopes.push(record.output);
  } else {
    // This is an explicit quality task, never static discovery. Native config
    // resolution is needed before a build has recorded its actual output.
    const words = pkg.scripts?.build?.trim().split(/\s+/) ?? [];
    let outDir = words.length === 4 && words[2] === '--outDir' ? words[3] : null;
    if (process.env.OYZU_NODE_VITE_STATE) {
      outDir = nativeRecord(process.env.OYZU_NODE_VITE_STATE).outDir;
    }
    const {resolveConfig} = await import(pathToFileURL(project.resolve('vite')).href);
    const config = await resolveConfig(outDir ? {build:{outDir}} : {}, 'build', 'production', 'production');
    scopes.push(relative(root, resolve(config.root, config.build.outDir)).split(sep).join('/'));
  }
}
if (!Array.isArray(scopes) || scopes.length > 1024 || scopes.some(p => typeof p !== 'string' || !p || isAbsolute(p) || /[\\:]/.test(p) || p.split('/').some(c => !c || c === '.' || c === '..'))) {
  throw new Error('Invalid workspace quality scope');
}
const excludedScopes = new Set(scopes);
const files = [];
function walk(directory) {
  for (const entry of readdirSync(directory, {withFileTypes:true}).sort((a,b) => a.name.localeCompare(b.name, 'en'))) {
    if (entry.isSymbolicLink() || excluded.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (excludedScopes.has(relative(root,path).split(sep).join('/'))) continue;
    if (entry.isDirectory()) walk(path);
    else if (entry.isFile() && /\.(?:[cm]?[jt]s|[jt]sx)$/.test(entry.name)) files.push(path);
    if (files.length > 100_000) throw new Error('Node quality source count exceeds limit');
  }
}
walk(root);
if (biome) {
  const metadata = locate('@biomejs/biome', '@biomejs/biome/package.json');
  const pkg = JSON.parse(readFileSync(metadata, 'utf8'));
  if (!pkg.version?.startsWith('2.')) throw new Error(`Biome ${pkg.version}: this adapter requires Biome 2`);
  if (typeof pkg.bin?.biome !== 'string') throw new Error('Missing native Biome entrypoint');
  const command = resolve(dirname(metadata), pkg.bin.biome);
  const flags = [mode === 'lint' ? 'lint' : 'format', '--files-ignore-unknown=true', '--no-errors-on-unmatched', '--colors=off'];
  if (mode === 'format') flags.push('--write');
  const invoke = batch => {
    const result = spawnSync(process.execPath, [command, ...flags, ...batch], {stdio:'inherit'});
    if (result.error) throw result.error;
    if (result.status !== 0) process.exitCode = result.status ?? 1;
  };
  // Bound host command lines while retaining native configuration and ignores.
  let batch = [], size = 0;
  for (const file of files) {
    const length = file.length * 2 + 4;
    if (length > 16_000) throw new Error('Biome source path exceeds host argument limit');
    if (size + length > 16_000) { invoke(batch); batch = []; size = 0; }
    batch.push(file); size += length;
  }
  if (batch.length) invoke(batch);
  console.log(`Biome ${pkg.version}: ${mode} (${files.length} source candidates)`);
} else if (mode === 'lint') {
  const eslint = library('eslint');
  const legacy = ['.eslintrc','.eslintrc.json','.eslintrc.js','.eslintrc.cjs','.eslintrc.yml','.eslintrc.yaml'].some(n => existsSync(join(root,n))) || pkg.eslintConfig != null;
  const major = Number(eslint.ESLint.version.split('.')[0]);
  if (legacy && major >= 10) throw new Error('Legacy ESLint configuration requires a captured ESLint 8/9 dependency or migration to flat configuration');
  const Class = eslint.loadESLint ? await eslint.loadESLint({useFlatConfig:!legacy}) : eslint.ESLint;
  const native = new Class({cwd:root, fix:false, cache:false});
  const results = [];
  if (legacy) {
    if (files.length) results.push(...await native.lintFiles(files));
  } else {
    const withoutConfig = [];
    for (const file of files) {
      if (await native.findConfigFile(file)) results.push(...await native.lintFiles([file]));
      else withoutConfig.push(file);
    }
    if (withoutConfig.length) {
      const js = library('@eslint/js');
      const globals = library('globals');
      const ts = library('typescript-eslint');
      const defaults = new Class({cwd:root, fix:false, cache:false, overrideConfigFile:true,
        overrideConfig:[js.configs.recommended,
          ...ts.configs.recommended.map(config => ({...config, files:['**/*.{ts,mts,cts,tsx}']})),
          {files:['**/*.{js,mjs,cjs,jsx,ts,mts,cts,tsx}'], languageOptions:{globals:{...globals.node,...(process.env.OYZU_NODE_BROWSER === '1' ? globals.browser : {})}, parserOptions:{ecmaFeatures:{jsx:true}}}},
          ...(process.env.OYZU_NODE_TEST_FRAMEWORK === 'jest' ? [{files:['**/*.{test,spec}.{js,cjs,mjs,ts,tsx,jsx}','**/__tests__/**'],languageOptions:{globals:globals.jest}}] : [])]});
      results.push(...await defaults.lintFiles(withoutConfig));
    }
  }
  const formatter = await native.loadFormatter('stylish');
  const output = formatter.format(results);
  if (output) process.stdout.write(output);
  if (results.some(r => r.errorCount || r.fatalErrorCount)) process.exitCode = 1;
  console.log(`ESLint ${eslint.ESLint.version}: checked ${files.length} source files (read-only)`);
} else {
  const prettier = library('prettier');
  const ignores = ['.gitignore','.prettierignore'].map(n => join(root,n)).filter(existsSync);
  for (const file of files) {
    const config = await prettier.resolveConfig(file, {editorconfig:true});
    const info = await prettier.getFileInfo(file, {ignorePath:ignores, plugins:config?.plugins});
    if (info.ignored || !info.inferredParser) continue;
    const source = readFileSync(file, 'utf8');
    const options = {...config, filepath:file};
    if (mode === 'format-check') {
      if (!await prettier.check(source, options)) {
        console.error(`Formatting differs: ${file}`);
        process.exitCode = 1;
      }
    } else {
      // Source walking never follows symlinks; check again before a write.
      if (!lstatSync(file).isFile() || lstatSync(file).isSymbolicLink()) throw new Error('Source changed during formatting');
      const formatted = await prettier.format(source, options);
      if (formatted !== source) writeFileSync(file, formatted);
    }
  }
  console.log(`Prettier ${prettier.version}: ${mode} (${files.length} source candidates)`);
}
