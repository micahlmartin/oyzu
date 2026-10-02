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
        shutil.copyfile(runtime/'quality.mjs', runtime/'node-quality.mjs')
        shutil.copyfile(ROOT/'src/broker/runtime/transport.mjs', runtime/'broker_transport.mjs')
        (runtime/'lock-probe.mjs').write_text("import {npm} from './npm-native.mjs'; import {mkdtempSync,rmSync} from 'node:fs'; import {tmpdir} from 'node:os'; import {join} from 'node:path'; const cache=mkdtempSync(join(tmpdir(),'oyzu-lock-')); try { npm(['install','--package-lock-only','--ignore-scripts'],process.cwd(),cache); } finally { rmSync(cache,{recursive:true,force:true}); }\n")
        snapshots = []
        for index in range(8):
            project = base/f'project {index}'
            shutil.copytree(ROOT/'examples/builds/node-workspace/project', project)
            (project/'operation.cjs').write_text("require('node:fs').appendFileSync(require('node:path').join(__dirname,'operations.log'),process.argv.slice(2).join(':')+'\\n');\n")
            for name in ['app', 'shared']:
                path = project/f'packages/{name}/package.json'
                pkg = json.loads(path.read_text())
                for stage in ['build', 'lint', 'format-check']:
                    pkg['scripts'][stage] = f'node ../../operation.cjs {stage} {name}'
                if index in [3,7]:
                    if name == 'app':
                        del pkg['scripts']['lint']
                        del pkg['scripts']['format-check']
                        del pkg['scripts']['test']
                    else:
                        pkg['scripts']['format:check'] = pkg['scripts'].pop('format-check')
                path.write_text(json.dumps(pkg))
            if index == 7:
                (project/'packages/app/biome.jsonc').write_text(json.dumps({'linter':{'rules':{'recommended':False,'suspicious':{'noDebugger':'error'}}}}))
            public_root = index == 2 or index >= 4
            if public_root:
                path = project/'package.json'
                pkg = json.loads(path.read_text())
                pkg.pop('private', None)
                pkg['dependencies'] = {'@oyzu-example/shared':'0.1.0'}
                if index == 6:
                    pkg['files'] = ['index.mjs', '.oyzu-build']
                path.write_text(json.dumps(pkg))
                (project/'index.mjs').write_text('export const value = 42;\n')
                (project/'root.test.mjs').write_text("import {test} from 'node:test'; import assert from 'node:assert/strict'; import {value} from './index.mjs'; test('root package',()=>assert.equal(value,42));\n")
            if index == 2:
                path = project/'package.json'
                pkg = json.loads(path.read_text())
                pkg['scripts'] = {s:f'node operation.cjs {s} root' for s in ['build','lint','format-check']}
                pkg['scripts']['test'] = 'node --test'
                pkg['scripts']['format:check'] = pkg['scripts'].pop('format-check')
                pkg['dependencies'] = {'@oyzu-example/shared':'0.1.0'}
                path.write_text(json.dumps(pkg))
            captured = base/f'capture {index}'
            captured.mkdir()
            env = {**os.environ, 'OYZU_NODE_QUALITY_HOME':str(ROOT/'tooling/images/node-quality')}
            # Author positive fixture sources with native Prettier before capture.
            subprocess.run(['node', str(ROOT/'tooling/images/node-quality/node_modules/prettier/bin/prettier.cjs'), '--write', *[str(p) for p in project.rglob('*') if p.suffix in ['.mjs','.cjs']]], check=True, capture_output=True)
            if index == 7:
                subprocess.run(['node',str(ROOT/'src/builders/node/runtime/quality.mjs'),'biome-format'],cwd=project/'packages/app',env=env,check=True,capture_output=True)

            def run(script, args, success=True):
                result = subprocess.run(['node', str(runtime/script), *args], cwd=project, env=env, capture_output=True, text=True, encoding='utf-8', timeout=120)
                assert (result.returncode == 0) == success, (args, result.stdout, result.stderr)
                return result

            if public_root:
                run('lock-probe.mjs', [])
            run('npm.mjs', ['acquire', str(captured), str(project), str(base/'absent-broker')])
            inventory = json.loads((captured/'inventory.json').read_text())
            version = '0.1.0-dev.gabcdef123456'
            members = sorted(inventory['workspaces']['members'], key=lambda m: len(m['dependencies']))
            quality = {'linter':'eslint', 'formatter':'prettier', 'excludes':[]}
            modules = [{**m, 'id':m['path'].split('/')[-1], 'version':version, 'filename':f"{m['name'].removeprefix('@').replace('/', '-')}-{version}.tgz", 'framework':'node-test', 'testExcludes':[], 'quality':quality} for m in members]
            if index == 7:
                next(m for m in modules if m['id']=='app')['quality'] = {'linter':'biome','formatter':'biome','excludes':[]}
            spec = {'rootVersion':version, 'rootDependencies':inventory['workspaces']['rootDependencies'], 'rootScripts':json.loads((project/'package.json').read_text()).get('scripts',{}), 'rootFramework':'node-test', 'rootQuality':quality, 'modules':modules, 'nodeTestArguments':['--experimental-test-coverage','--test-coverage-exclude=**/*.test.*','--test-reporter=junit','--test-reporter-destination=__OYZU_TEST_REPORT__','--test-reporter=lcov','--test-reporter-destination=__OYZU_COVERAGE_REPORT__']}
            spec['rootQuality'] = {**quality, 'excludes':[m['path'] for m in members]}
            if public_root:
                spec['rootArtifact'] = {'id':'root-package', 'path':'.', 'name':'oyzu-workspace', 'version':version, 'filename':f'oyzu-workspace-{version}.tgz'}
            encoded = json.dumps(spec)
            env['OYZU_NODE_WORKSPACE_PLAN'] = hashlib.sha256(encoded.encode()).hexdigest()
            env['OYZU_TARGET'] = 'workspace'

            def operation(mode, success=True):
                return run('npm-workspace-build.mjs', [mode, '', str(base/f'output {index}')], success)

            run('npm-workspace-build.mjs', ['project', encoded, str(captured)])
            if public_root and index != 2:
                (project/'.oyzu-build/leak.test.mjs').write_text("throw Error('engine state must not be tested or packed');\n")
            run('npm.mjs', ['install', str(captured), str(project)])
            operation('build')
            operation('test')
            operation('lint')
            operation('format-check')
            expected = ([f'{stage}:root' for stage in ['build','lint','format-check']] if index == 2 else [f'{stage}:{member}' for stage in ['build','lint','format-check'] for member in ['shared','app'] if index not in [3,7] or member == 'shared' or stage == 'build'])
            assert (project/'operations.log').read_text().splitlines() == expected
            if index in [3,7]:
                # A scripted sibling must not suppress the other member's checks.
                bad = project/'packages/app/invalid.mjs'
                bad.write_text('debugger;\n' if index == 7 else 'export const invalid = absent;\n', newline='\n')
                result = operation('lint', False)
                assert ('noDebugger' if index == 7 else 'no-undef') in result.stdout + result.stderr
                bad.write_text('export const valid=1;\n', newline='\n')
                result = operation('format-check', False)
                assert ('format' if index == 7 else 'Formatting differs') in result.stdout + result.stderr
                bad.unlink()
            for module in (['root'] if index == 2 else ['app','shared'] + (['root'] if public_root else [])):
                report = project/f'.oyzu-build/reports/{module}'
                junit = ET.parse(report/'junit.xml')
                assert len(junit.findall('.//testcase')) == (3 if index == 2 else 1)
                assert not junit.findall('.//failure')
                assert 'DA:' in (report/'coverage.lcov').read_text()
            operation('package')
            artifacts = base/f'output {index}/workspace/artifacts'
            assert len(list(artifacts.glob('*.tgz'))) == (3 if public_root else 2)
            snapshot = {}
            for artifact in artifacts.glob('*.tgz'):
                snapshot[artifact.name] = hashlib.sha256(artifact.read_bytes()).hexdigest()
                with tarfile.open(artifact) as tar:
                    pkg = json.load(tar.extractfile('package/package.json'))
                    assert pkg['version'] == version
                    if pkg['name'].endswith('/app'):
                        assert pkg['dependencies']['@oyzu-example/shared'] == version
                    if pkg['name'] == 'oyzu-workspace':
                        assert pkg['dependencies']['@oyzu-example/shared'] == version
                        assert 'package/index.mjs' in tar.getnames()
                        assert not any('/.oyzu-build/' in n or '/.oyzu/' in n for n in tar.getnames())
                        if index == 6:
                            assert sorted(tar.getnames()) == ['package/index.mjs','package/package.json']
            snapshots.append(snapshot)
            if public_root and index != 2:
                primary = project/'root.test.mjs'
                original = primary.read_text()
                primary.write_text("import {test} from 'node:test'; test('root failure',()=>{throw Error('expected')});\n")
                operation('test', False)
                assert ET.parse(project/'.oyzu-build/reports/root/junit.xml').findall('.//failure')
                primary.unlink()
                assert 'No package Node tests found' in operation('test', False).stderr
                primary.write_text(original)
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
        assert snapshots[4] == snapshots[5], 'root package bytes must survive relocation'
    print('Native npm workspace builds: snapshot packs, internal versions, stable bytes, dependency order, root deduplication, mixed scripted/default quality, JUnit/coverage, failures and tamper rejection passed')


if __name__ == '__main__':
    main()
