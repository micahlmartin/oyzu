"""Exercise native workspace build/package/report behavior without a mock npm."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix='oyzu workspace build ') as temporary:
        base = Path(temporary)
        runtime = base/'runtime'
        shutil.copytree(ROOT/'src/builders/node/runtime', runtime)
        shutil.copyfile(runtime/'lock.mjs', runtime/'npm_lock.mjs')
        shutil.copyfile(ROOT/'src/broker/runtime/transport.mjs', runtime/'broker_transport.mjs')
        (runtime/'lock-probe.mjs').write_text("import {npm} from './npm-native.mjs'; import {mkdtempSync,rmSync} from 'node:fs'; import {tmpdir} from 'node:os'; import {join} from 'node:path'; const cache=mkdtempSync(join(tmpdir(),'oyzu-lock-')); try { npm(['install','--package-lock-only','--ignore-scripts'],process.cwd(),cache); } finally { rmSync(cache,{recursive:true,force:true}); }\n")
        snapshots = []
        for index in range(3):
            project = base/f'project {index}'
            shutil.copytree(ROOT/'examples/builds/node-workspace/project', project)
            (project/'operation.cjs').write_text("require('node:fs').appendFileSync(require('node:path').join(__dirname,'operations.log'),process.argv.slice(2).join(':')+'\\n');\n")
            for name in ['app', 'shared']:
                path = project/f'packages/{name}/package.json'
                pkg = json.loads(path.read_text())
                for stage in ['build', 'lint', 'format-check']:
                    pkg['scripts'][stage] = f'node ../../operation.cjs {stage} {name}'
                path.write_text(json.dumps(pkg))
            if index == 2:
                path = project/'package.json'
                pkg = json.loads(path.read_text())
                pkg['scripts'] = {s:f'node operation.cjs {s} root' for s in ['build','lint','format-check']}
                pkg['scripts']['test'] = 'node --test'
                pkg['dependencies'] = {'@oyzu-example/shared':'0.1.0'}
                path.write_text(json.dumps(pkg))
            captured = base/f'capture {index}'
            captured.mkdir()
            env = os.environ.copy()

            def run(script, args, success=True):
                result = subprocess.run(['node', str(runtime/script), *args], cwd=project, env=env, capture_output=True, text=True, timeout=120)
                assert (result.returncode == 0) == success, (args, result.stdout, result.stderr)
                return result

            if index == 2:
                run('lock-probe.mjs', [])
            run('npm.mjs', ['acquire', str(captured), str(project), str(base/'absent-broker')])
            inventory = json.loads((captured/'inventory.json').read_text())
            version = '0.1.0-dev.gabcdef123456'
            members = sorted(inventory['workspaces']['members'], key=lambda m: len(m['dependencies']))
            modules = [{**m, 'id':m['path'].split('/')[-1], 'version':version, 'filename':f"{m['name'].removeprefix('@').replace('/', '-')}-{version}.tgz", 'framework':'node-test'} for m in members]
            spec = {'rootVersion':version, 'rootDependencies':inventory['workspaces']['rootDependencies'], 'rootScripts':json.loads((project/'package.json').read_text()).get('scripts',{}), 'rootFramework':'node-test', 'modules':modules, 'nodeTestArguments':['--experimental-test-coverage','--test-coverage-exclude=**/*.test.*','--test-reporter=junit','--test-reporter-destination=__OYZU_TEST_REPORT__','--test-reporter=lcov','--test-reporter-destination=__OYZU_COVERAGE_REPORT__']}
            encoded = json.dumps(spec)
            env['OYZU_NPM_WORKSPACE_PLAN'] = hashlib.sha256(encoded.encode()).hexdigest()
            env['OYZU_TARGET'] = 'workspace'

            def operation(mode, success=True):
                return run('npm-workspace-build.mjs', [mode, '', str(base/f'output {index}')], success)

            run('npm-workspace-build.mjs', ['project', encoded, str(captured)])
            run('npm.mjs', ['install', str(captured), str(project)])
            operation('build')
            operation('test')
            operation('lint')
            operation('format-check')
            assert (project/'operations.log').read_text().splitlines() == ([f'{stage}:root' for stage in ['build','lint','format-check']] if index == 2 else [f'{stage}:{member}' for stage in ['build','lint','format-check'] for member in ['shared','app']])
            for module in (['root'] if index == 2 else ['app','shared']):
                report = project/f'.oyzu-build/reports/{module}'
                junit = ET.parse(report/'junit.xml')
                assert len(junit.findall('.//testcase')) == (2 if index == 2 else 1)
                assert not junit.findall('.//failure')
                assert 'DA:' in (report/'coverage.lcov').read_text()
            operation('package')
            artifacts = base/f'output {index}/workspace/artifacts'
            assert len(list(artifacts.glob('*.tgz'))) == 2
            snapshot = {}
            for artifact in artifacts.glob('*.tgz'):
                snapshot[artifact.name] = hashlib.sha256(artifact.read_bytes()).hexdigest()
                with tarfile.open(artifact) as tar:
                    pkg = json.load(tar.extractfile('package/package.json'))
                    assert pkg['version'] == version
                    if pkg['name'].endswith('/app'):
                        assert pkg['dependencies']['@oyzu-example/shared'] == version
            snapshots.append(snapshot)
            member = modules[0]
            artifact = project/'.oyzu-build/artifacts'/member['filename']
            artifact.write_bytes(artifact.read_bytes()+b'changed')
            assert 'changed after build' in operation('package', False).stderr
            (project/'packages/shared/failing.test.mjs').write_text("import {test} from 'node:test'; test('native failure',()=>{throw Error('expected')});\n")
            operation('test', False)
            report = project/f".oyzu-build/reports/{'root' if index == 2 else 'shared'}/junit.xml"
            assert ET.parse(report).findall('.//failure')
            plan = project/'.oyzu-build/plan.json'
            plan.write_text('{}')
            assert 'plan identity changed' in operation('build', False).stderr
        assert snapshots[0] == snapshots[1], 'workspace package bytes must survive relocation'
    print('Native npm workspace builds: snapshot packs, internal versions, stable bytes, dependency order, root deduplication, JUnit/coverage, failures and tamper rejection passed')


if __name__ == '__main__':
    main()
