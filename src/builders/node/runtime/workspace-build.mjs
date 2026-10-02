// Shared package operations. Native managers own version projection and packing. The engine still owns hooks and evidence collection.
import {spawnSync} from 'node:child_process';
import {copyFileSync, constants, mkdirSync, writeFileSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {digest, read, regular, root, specification, state} from './workspace-plan.mjs';
import {testWorkspace} from './workspace-testing.mjs';

export function runWorkspace(profile) {
const [mode, encoded, location] = process.argv.slice(2);
if (mode === 'project') {
  profile.project(encoded, location);
} else {
  const spec = specification();
  const artifactDirectory = join(state, 'artifacts');
  const receipt = join(state, 'artifacts.json');
  const packages = [...spec.modules, ...(spec.rootArtifact ? [spec.rootArtifact] : [])];
  const runtime = dirname(fileURLToPath(import.meta.url));
  const invoke = (command, cwd, env = process.env) => {
    const result = spawnSync(command[0], command.slice(1), {cwd, env, stdio:'inherit'});
    if (result.error) throw result.error;
    return result.status ?? 1;
  };
  const script = profile.script;
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
    for (const member of packages) {
      const pkg = read(join(root, member.path, 'package.json'));
      if (pkg.name !== member.name || pkg.version !== member.version) throw new Error('Build changed planned package identity');
      profile.pack(member, artifactDirectory);
      artifacts.push({filename:member.filename, sha256:digest(regular(join(artifactDirectory, member.filename)))});
    }
    writeFileSync(receipt, JSON.stringify(artifacts), {flag:'wx'});
  } else if (mode === 'test') {
    process.exitCode = testWorkspace(spec, {root, reportRoot:join(state, 'reports'), script});
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
        const checker = member.quality?.[lint ? 'linter' : 'formatter'];
        if (![lint ? 'eslint' : 'prettier', 'biome'].includes(checker)) {
          throw new Error(`Unsupported implicit workspace quality tool in ${member.path}`);
        }
        const excludes = member.quality.excludes;
        if (!Array.isArray(excludes)) throw new Error('Missing planned workspace quality scope');
        const env = {...process.env, OYZU_NODE_TEST_FRAMEWORK:member.framework, OYZU_NODE_QUALITY_EXCLUDE:JSON.stringify(excludes)};
        const operation = lint ? 'lint' : 'format-check';
        if (invoke([process.execPath, join(runtime, 'node-quality.mjs'), checker === 'biome' ? `biome-${operation}` : operation], resolve(root,member.path), env)) process.exitCode = 1;
      }
    }
  } else if (mode === 'package') {
    const artifacts = read(receipt);
    if (artifacts.length !== packages.length) throw new Error('Incomplete workspace artifact receipt');
    const output = join(location ?? '/out', process.env.OYZU_TARGET, 'artifacts');
    mkdirSync(output, {recursive:true});
    for (const member of packages) {
      const artifact = artifacts.find(a => a.filename === member.filename);
      const source = join(artifactDirectory, member.filename);
      if (!artifact || digest(regular(source)) !== artifact.sha256) throw new Error('Workspace artifact changed after build');
      copyFileSync(source, join(output, member.filename), constants.COPYFILE_EXCL);
    }
  } else throw new Error(`Unknown workspace operation: ${mode}`);
}
}
