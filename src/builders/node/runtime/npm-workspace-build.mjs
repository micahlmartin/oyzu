// Native npm workspace operations. The engine still owns hooks and evidence collection.
import {spawnSync} from 'node:child_process';
import {copyFileSync, constants, mkdirSync, mkdtempSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {createRequire} from 'node:module';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {npm, npmCommand} from './npm-native.mjs';
import {digest, project, read, regular, root, specification, state} from './npm-workspace-plan.mjs';

const [mode, encoded, location] = process.argv.slice(2);
if (mode === 'project') {
  project(encoded, location);
} else {
  const spec = specification();
  const cache = mkdtempSync(join(tmpdir(), 'oyzu-npm-workspace-'));
  try {
  const artifactDirectory = join(state, 'artifacts');
  const receipt = join(state, 'artifacts.json');
  const runtime = dirname(fileURLToPath(import.meta.url));
  const invoke = (command, cwd, env = process.env) => {
    const result = spawnSync(command[0], command.slice(1), {cwd, env, stdio:'inherit'});
    if (result.error) throw result.error;
    return result.status ?? 1;
  };
  const script = (name, member) => npmCommand(['run', name, ...(member ? ['--workspace', member.name] : []), '--'], cache);
  if (mode === 'build') {
    if (spec.rootScripts.build) {
      if (invoke(script('build'), root)) throw new Error('Workspace root build failed');
    } else {
      for (const member of spec.modules) {
        if (member.scripts.build && invoke(script('build', member), root)) throw new Error(`Workspace build failed: ${member.name}`);
      }
    }
    mkdirSync(artifactDirectory);
    const artifacts = [];
    for (const member of spec.modules) {
      const pkg = read(join(root, member.path, 'package.json'));
      if (pkg.name !== member.name || pkg.version !== member.version) throw new Error('Build changed planned package identity');
      const packed = JSON.parse(npm(['pack', '--ignore-scripts', '--json', '--workspace', member.name, '--pack-destination', artifactDirectory], root, cache));
      if (packed.length !== 1 || packed[0].filename !== member.filename || packed[0].name !== member.name || packed[0].version !== member.version) throw new Error('Native npm pack differs from planned artifact');
      artifacts.push({filename:member.filename, sha256:digest(regular(join(artifactDirectory, member.filename)))});
    }
    writeFileSync(receipt, JSON.stringify(artifacts), {flag:'wx'});
  } else if (mode === 'test') {
    const producers = spec.rootScripts.test
      ? [{id:'root', path:'.', scripts:spec.rootScripts, framework:spec.rootFramework}]
      : spec.modules;
    for (const member of producers) {
      const reports = join(state, 'reports', member.id);
      mkdirSync(reports, {recursive:true});
      const env = {...process.env, OYZU_TEST_REPORT:join(reports, 'junit.xml'), OYZU_COVERAGE_REPORT:join(reports, 'coverage.lcov')};
      const cwd = resolve(root, member.path);
      let command;
      if (member.scripts.test) command = script('test', member.name ? member : undefined);
      else if (member.framework === 'node-test') command = [process.execPath, '--test'];
      else if (['jest', 'vitest'].includes(member.framework)) {
        const native = createRequire(join(cwd, 'package.json'));
        command = [process.execPath, native.resolve(member.framework === 'jest' ? 'jest/bin/jest' : 'vitest/vitest.mjs')];
      } else throw new Error(`No native test command for ${member.name}`);
      if (member.framework === 'node-test') {
        command.push(...spec.nodeTestArguments.map(v => v.replace('__OYZU_TEST_REPORT__', env.OYZU_TEST_REPORT).replace('__OYZU_COVERAGE_REPORT__', env.OYZU_COVERAGE_REPORT)));
      } else if (['jest', 'vitest'].includes(member.framework)) {
        command = [process.execPath, join(runtime, `${member.framework}.mjs`), ...command];
      }
      const status = invoke(command, member.scripts.test ? root : cwd, env);
      if (status) process.exitCode = 1;
    }
  } else if (['lint', 'format-check', 'format:check'].includes(mode)) {
    const aliases = mode === 'lint' ? ['lint'] : ['format-check','format:check'];
    const rootScript = aliases.find(name => Object.hasOwn(spec.rootScripts, name));
    if (rootScript) {
      if (invoke(script(rootScript), root)) process.exitCode = 1;
    } else {
      const producers = [{path:'.', scripts:{}, framework:spec.rootFramework, quality:spec.rootQuality}, ...spec.modules];
      for (const member of producers) {
        const name = aliases.find(name => Object.hasOwn(member.scripts, name));
        if (name) {
          if (invoke(script(name, member), root)) process.exitCode = 1;
          continue;
        }
        const lint = mode === 'lint';
        if (member.quality?.[lint ? 'linter' : 'formatter'] !== (lint ? 'eslint' : 'prettier')) {
          throw new Error(`Unsupported implicit workspace quality tool in ${member.path}`);
        }
        const prefix = member.path === '.' ? '' : `${member.path}/`;
        const excludes = spec.modules.map(m => m.path).filter(p => p.startsWith(prefix) && p !== member.path).map(p => p.slice(prefix.length));
        const env = {...process.env, OYZU_NODE_TEST_FRAMEWORK:member.framework, OYZU_NODE_QUALITY_EXCLUDE:JSON.stringify(excludes)};
        if (invoke([process.execPath, join(runtime, 'node-quality.mjs'), lint ? 'lint' : 'format-check'], resolve(root,member.path), env)) process.exitCode = 1;
      }
    }
  } else if (mode === 'package') {
    const artifacts = read(receipt);
    if (artifacts.length !== spec.modules.length) throw new Error('Incomplete workspace artifact receipt');
    const output = join(location ?? '/out', process.env.OYZU_TARGET, 'artifacts');
    mkdirSync(output, {recursive:true});
    for (const member of spec.modules) {
      const artifact = artifacts.find(a => a.filename === member.filename);
      const source = join(artifactDirectory, member.filename);
      if (!artifact || digest(regular(source)) !== artifact.sha256) throw new Error('Workspace artifact changed after build');
      copyFileSync(source, join(output, member.filename), constants.COPYFILE_EXCL);
    }
  } else throw new Error(`Unknown npm workspace operation: ${mode}`);
  } finally {
    rmSync(cache, {recursive:true, force:true});
  }
}
