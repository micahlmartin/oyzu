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
            result = subprocess.run([str(cli), '-C', str(project), *args], capture_output=True, text=True, timeout=120, env=env)
            assert (result.returncode == 0) == success, result.stdout + result.stderr
            return result

        tasks = json.loads(run('run','list','--json',env={**os.environ,'PATH':''}).stdout)
        assert tasks['project:build']['availability'] is None
        assert tasks['project:build']['build_stage'] is True
        assert not log.exists(), 'discovery executed a project script'
        # Provision only local workspace links using the already installed npm.
        module = (ROOT/'src/builders/node/runtime/npm-native.mjs').as_uri()
        subprocess.run(['node','--input-type=module','-e',f"import {{npm}} from {json.dumps(module)}; npm(['install','--ignore-scripts'],process.cwd(),process.argv[1]);",str(base/'cache')],cwd=project,check=True,capture_output=True,text=True)
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
    print('Compiled CLI npm workspace tasks: dependency order, native pre/post lifecycle, exact arguments, failed producer blocking, root ownership, overrides and nonexecuting discovery passed')


if __name__ == '__main__':
    main()
