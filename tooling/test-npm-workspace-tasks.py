"""Run actual compiled-CLI development tasks against a native npm workspace."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    cli = parser.parse_args().cli.resolve()
    with tempfile.TemporaryDirectory(prefix='oyzu workspace tasks ') as temporary:
        base = Path(temporary)
        project = base/'project'
        shutil.copytree(ROOT/'examples/builds/node-workspace/project', project)
        log = project/'operations.jsonl'
        for member in ['app', 'shared']:
            manifest = project/f'packages/{member}/package.json'
            package = json.loads(manifest.read_text())
            package['scripts'].update({stage:f'node operation.cjs {stage}' for stage in ['prebuild','build','postbuild']})
            manifest.write_text(json.dumps(package))
            (manifest.parent/'operation.cjs').write_text("const fs=require('node:fs'); const path=require('node:path'); const name=require('./package.json').name; const args=process.argv.slice(2); if(args[0]==='build' && name.endsWith('/app') && !fs.existsSync('../shared/built')) throw Error('dependency not built'); fs.appendFileSync('../../operations.jsonl',JSON.stringify({name,args})+'\\n'); if(args[0]==='build') fs.writeFileSync('built','yes'); if(process.env.FAIL_SHARED==='yes' && name.endsWith('/shared') && args[0]==='build') process.exit(7);\n")

        def run(*args, success=True, env=None):
            environment = {**os.environ, 'OYZU_NODE_QUALITY_HOME':str(ROOT/'tooling/images/node-quality'), **(env or {})}
            result = subprocess.run([str(cli), '-C', str(project), *args], capture_output=True, text=True, encoding='utf-8', timeout=120, env=environment)
            assert (result.returncode == 0) == success, result.stdout + result.stderr
            return result

        tasks = json.loads(run('run','list','--json',env={**os.environ,'PATH':''}).stdout)
        assert tasks['project:build']['availability'] is None
        assert tasks['project:build']['build_stage'] is True
        assert not log.exists(), 'discovery executed a project script'
        # Provision only local workspace links using the already installed npm.
        module = (ROOT/'src/builders/node/runtime/npm-native.mjs').as_uri()
        def install_native():
            subprocess.run(['node','--input-type=module','-e',f"import {{npm}} from {json.dumps(module)}; npm(['install','--ignore-scripts'],process.cwd(),process.argv[1]);",str(base/'cache')],cwd=project,check=True,capture_output=True,text=True)
        install_native()
        forwarded = ['argument with spaces', '', '--literal', 'semi;colon', 'snowman-\u2603']
        run('run','build','--',*forwarded)
        operations = [json.loads(line) for line in log.read_text(encoding='utf-8').splitlines()]
        assert [(op['name'].split('/')[-1],op['args'][0]) for op in operations] == [(member,stage) for member in ['shared','app'] for stage in ['prebuild','build','postbuild']]
        assert all(op['args'][1:] == (forwarded if op['args'][0]=='build' else []) for op in operations), operations
        log.unlink()
        manifest = project/'packages/shared/package.json'
        original = manifest.read_text()
        package = json.loads(original)
        package['dependencies'] = {'@oyzu-example/app':'0.1.0'}
        manifest.write_text(json.dumps(package))
        assert 'dependency cycle' in run('run','build',success=False).stderr
        assert not log.exists(), 'cycle executed a member script'
        manifest.write_text(original)
        run('run','build',success=False,env={**os.environ,'FAIL_SHARED':'yes'})
        operations = [json.loads(line) for line in log.read_text(encoding='utf-8').splitlines()]
        assert [(op['name'].split('/')[-1],op['args'][0]) for op in operations] == [('shared','prebuild'),('shared','build')]
        log.unlink()
        # The root's explicit native script remains authoritative.
        manifest = project/'package.json'
        package = json.loads(manifest.read_text())
        package['scripts'] = {'build':'node root.cjs'}
        manifest.write_text(json.dumps(package))
        (project/'root.cjs').write_text("require('node:fs').writeFileSync('root-ran','yes');\n")
        run('run','build')
        assert (project/'root-ran').read_text() == 'yes' and not log.exists()
        # A user override must not be interpreted as a native aggregate task.
        (project/'oyzu.toml').write_text('[tasks."project:build"]\nargv=["node","root.cjs"]\n')
        run('run','project:build')
        assert not log.exists()
        (project/'oyzu.toml').unlink()
        package.pop('scripts')
        manifest.write_text(json.dumps(package))
        for member in ['app','shared']:
            member_manifest = project/f'packages/{member}/package.json'
            package = json.loads(member_manifest.read_text())
            package.pop('scripts')
            member_manifest.write_text(json.dumps(package))
        result = run('run','build')
        assert result.stdout.count('no declared build script') == 2 and not log.exists()
        for member in ['app','shared']:
            (project/f'packages/{member}/operation.cjs').unlink()
        sources = [p for p in project.rglob('*') if p.suffix in ['.mjs','.cjs'] and 'node_modules' not in p.parts]
        subprocess.run(['node',str(ROOT/'tooling/images/node-quality/node_modules/prettier/bin/prettier.cjs'),'--write',*map(str,sources)],check=True,capture_output=True,text=True)
        shared_manifest = project/'packages/shared/package.json'
        package = json.loads(shared_manifest.read_text())
        package['scripts'] = {'lint':'node --check index.mjs', 'format:check':'node --check index.mjs'}
        shared_manifest.write_text(json.dumps(package))
        # A scripted sibling's source must not be inspected by root defaults.
        ignored = project/'packages/shared/ignored.mjs'
        ignored.write_text('this is not valid JavaScript\n')
        tasks = json.loads(run('run','list','--json').stdout)
        assert 'project:format-check' in tasks and 'project:format:check' not in tasks
        assert tasks['project:format']['mutates_source'] is True
        assert tasks['project:format']['build_stage'] is False
        run('run','lint')
        run('run','format-check')
        nested = project/'packages/app/nested'
        nested.mkdir()
        (nested/'package.json').write_text(json.dumps({'name':'@oyzu-example/nested','version':'0.1.0','private':True,'scripts':{'lint':'node --check index.mjs','format:check':'node --check index.mjs','format':'node --check index.mjs'}}))
        (nested/'index.mjs').write_text('export const nested = true;\n',newline='\n')
        (nested/'ignored.mjs').write_text('this is not valid JavaScript\n')
        package = json.loads(manifest.read_text())
        package['workspaces'].append('packages/app/nested')
        manifest.write_text(json.dumps(package))
        install_native()
        # Root and parent defaults must both honor the nested member's scope.
        run('run','lint')
        run('run','format-check')
        invalid = project/'packages/app/invalid.mjs'
        invalid.write_text('export const value = absent;\n',newline='\n')
        assert 'no-undef' in run('run','lint',success=False).stdout
        invalid.write_text('export const value=1;\n',newline='\n')
        before = invalid.read_bytes()
        assert 'Formatting differs' in run('run','format-check',success=False).stderr
        assert invalid.read_bytes() == before
        # Explicit mutation is separate and applies the same package ownership.
        ignored.unlink()
        run('run','format')
        assert invalid.read_bytes() != before
        run('run','format-check')
        invalid.unlink()
        # Failure in root-owned source must not suppress member checks.
        (project/'invalid.mjs').write_text('export const root = absent;\n',newline='\n')
        invalid.write_text('export const member = missing;\n',newline='\n')
        result = run('run','lint',success=False)
        assert 'absent' in result.stdout and 'missing' in result.stdout
        (project/'invalid.mjs').unlink()
        invalid.unlink()
        assert 'do not accept extra arguments' in run('run','lint','--','--fix',success=False).stderr
        # An explicit root formatting alias owns the operation once.
        package = json.loads(manifest.read_text())
        package['scripts'] = {'format:check':'node root.cjs'}
        manifest.write_text(json.dumps(package))
        tasks = json.loads(run('run','list','--json').stdout)
        assert 'project:format:check' in tasks and 'project:format-check' not in tasks
        run('run','format:check')
        # A member can select Biome while its root uses defaults and siblings own scripts.
        package.pop('scripts')
        manifest.write_text(json.dumps(package))
        (project/'packages/app/biome.jsonc').write_text('// Native member config\n'+json.dumps({'linter':{'rules':{'recommended':False,'suspicious':{'noDebugger':'error'}}}}))
        run('run','format')
        run('run','lint')
        run('run','format-check')
        invalid.write_text('debugger;\n',newline='\n')
        result = run('run','lint',success=False)
        assert 'noDebugger' in result.stdout + result.stderr
        invalid.unlink()
    print('Compiled CLI npm workspace tasks: dependency order, native lifecycle, exact arguments, failures, root ownership, overrides, nonexecuting discovery, mixed quality, nested scopes, format aliases and explicit formatting passed')


if __name__ == '__main__':
    main()
