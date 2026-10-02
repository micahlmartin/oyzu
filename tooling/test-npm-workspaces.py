"""Native npm workspace capture/replay; no simulated resolver or package manager."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix='oyzu npm workspace ') as temporary:
        base = Path(temporary)
        runtime = base/'runtime'
        runtime.mkdir()
        for name in ['npm.mjs', 'npm-native.mjs', 'npm-workspaces.mjs', 'integrity.mjs', 'node-runtime.mjs']:
            shutil.copyfile(ROOT/'src/builders/node/runtime'/name, runtime/name)
        shutil.copyfile(ROOT/'src/builders/node/runtime/lock.mjs', runtime/'npm_lock.mjs')
        shutil.copyfile(ROOT/'src/broker/runtime/transport.mjs', runtime/'broker_transport.mjs')
        # No broker is provisioned: these workspaces contain only local edges.
        def execute(mode, output, project, success=True):
            result = subprocess.run(['node', str(runtime/'npm.mjs'), mode, str(output), str(project), str(base/'no-broker')], capture_output=True, text=True, timeout=120)
            assert (result.returncode==0)==success, (result.stdout, result.stderr)
            return result
        captures = []
        for index in range(2):
            project = base/f'project {index}'
            shutil.copytree(ROOT/'examples/builds/node-workspace/project', project)
            shared = project/'packages/shared'
            manifest = shared/'package.json'
            package = json.loads(manifest.read_text())
            package['scripts']['preinstall'] = 'node lifecycle.cjs'
            manifest.write_text(json.dumps(package))
            (shared/'lifecycle.cjs').write_text("require('node:fs').writeFileSync('lifecycle-ran','yes');\n")
            control = {p.relative_to(project).as_posix(): p.read_bytes() for p in project.rglob('*.json')}
            output = base/f'capture {index}'
            output.mkdir()
            execute('acquire', output, project)
            assert not (shared/'lifecycle-ran').exists()
            for name, contents in control.items():
                assert (project/name).read_bytes()==contents
            captures.append({p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in output.rglob('*') if p.is_file()})
            inventory = json.loads((output/'inventory.json').read_text())
            assert inventory['packages']==[]
            members = inventory['workspaces']['members']
            assert [m['name'] for m in members]==['@oyzu-example/app', '@oyzu-example/shared']
            assert members[0]['private'] and not members[1]['private']
            assert members[0]['dependencies']==[{'name':'@oyzu-example/shared', 'target':'@oyzu-example/shared', 'kind':'prod', 'spec':'0.1.0'}]
            assert members[1]['dependencies']==[]
            execute('install', output, project)
            assert (shared/'lifecycle-ran').read_text()=='yes'
            result = subprocess.run(['node','--test','--test-reporter=tap'], cwd=project, capture_output=True, text=True)
            assert result.returncode==0 and '# pass 2' in result.stdout, (result.stdout,result.stderr)
        assert captures[0]==captures[1], 'workspace capture must not depend on absolute paths or cache state'
        if os.name != 'nt':
            alias = base/'aliased root'
            alias.symlink_to(project, target_is_directory=True)
            output = base/'aliased capture'
            output.mkdir()
            execute('acquire', output, alias)
            assert json.loads((output/'inventory.json').read_text())==inventory
            execute('install', output, alias)
        package['version'] = '2.0.0'
        manifest.write_text(json.dumps(package))
        output = base/'stale'
        output.mkdir()
        assert 'missing or stale' in execute('acquire', output, project, False).stderr
        package['version'] = '0.1.0'
        manifest.write_text(json.dumps(package))
        lock = project/'package-lock.json'
        value = json.loads(lock.read_text())
        value['packages']['node_modules/@oyzu-example/shared']['resolved'] = '../outside'
        lock.write_text(json.dumps(value))
        output = base/'escape'
        output.mkdir()
        assert 'not a captured native workspace' in execute('acquire', output, project, False).stderr
        lock.unlink()
        output = base/'unlocked'
        output.mkdir()
        assert 'require a captured lockfile' in execute('acquire', output, project, False).stderr
        root_package = project/'package.json'
        root_package.write_text(json.dumps({'workspaces':['../outside']}))
        assert 'must stay within captured source' in execute('acquire', output, project, False).stderr
        root_package.write_text(json.dumps({'name':'public-root','version':'invalid','workspaces':['packages/*']}))
        assert 'requires a valid native name/version' in execute('acquire', output, project, False).stderr
    print('Native npm workspaces: stable module/edge capture, private flags, offline link replay, native tests, lifecycle isolation and stale/escaping input rejection passed')


if __name__=='__main__':
    main()
