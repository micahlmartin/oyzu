// Native quality APIs; usable as a runtime file or embedded development command.
import {existsSync, lstatSync, readFileSync, readdirSync, writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {join, resolve} from 'node:path';

const root = process.cwd();
const mode = process.argv.at(-1);
if (!['lint', 'format-check', 'format'].includes(mode)) throw new Error('Expected Node quality operation');
const project = createRequire(join(root, 'package.json'));
const fallback = process.env.OYZU_NODE_QUALITY_HOME
  ? createRequire(join(resolve(process.env.OYZU_NODE_QUALITY_HOME), 'package.json')) : null;
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
function library(name) {
  let resolved;
  try { resolved = project.resolve(name); }
  catch (error) {
    if (error.code !== 'MODULE_NOT_FOUND') throw error;
    if (['dependencies','devDependencies','optionalDependencies'].some(k => Object.hasOwn(pkg[k] ?? {}, name))) {
      throw new Error(`Declared ${name} is not installed; prepare the project's native dependencies first`);
    }
    if (!fallback) throw new Error(`No provisioned ${name}; supply project dependencies or OYZU_NODE_QUALITY_HOME`);
    resolved = fallback.resolve(name);
  }
  return project(resolved);
}
const excluded = new Set(['node_modules','.git','.oyzu','.oyzu-build','dist','build','coverage']);
const files = [];
function walk(directory) {
  for (const entry of readdirSync(directory, {withFileTypes:true}).sort((a,b) => a.name.localeCompare(b.name, 'en'))) {
    if (entry.isSymbolicLink() || excluded.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) walk(path);
    else if (entry.isFile() && /\.(?:[cm]?[jt]s|[jt]sx)$/.test(entry.name)) files.push(path);
    if (files.length > 100_000) throw new Error('Node quality source count exceeds limit');
  }
}
walk(root);
if (mode === 'lint') {
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
          {files:['**/*.{js,mjs,cjs,jsx,ts,mts,cts,tsx}'], languageOptions:{globals:globals.node, parserOptions:{ecmaFeatures:{jsx:true}}}},
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
