"""Native pip installation, runnable Python archives and artifact-source coverage."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT/'src/builders/python/runtime'
spec = importlib.util.spec_from_file_location('adapter', RUNTIME/'adapter.py')
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--wheels', type=Path, required=True)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='oyzu python application ') as temporary:
        base = Path(temporary)
        dependencies = base/'dependencies'
        shutil.copytree(args.wheels, dependencies/'wheels')
        adapter.inventory(
            {'packaging':'runtime', 'pytest':'test', 'pytest-cov':'test', 'ruff':'test'},
            ['packaging==24.2', 'pytest==8.3.5', 'pytest-cov==6.0.0', 'ruff==0.11.13'],
            dependencies,
        )
        inventory = json.loads((dependencies/'packages.json').read_text())
        assert inventory['runtimeRoots']==['packaging/24.2']
        assert next(p for p in inventory['packages'] if p['name']=='pytest-cov')['dependencies']
        hashes = []
        env = {**os.environ, 'OYZU_VERSION':'0.0.0-dev.g0123456789ab', 'OYZU_SOURCE_DIGEST':'sha256:'+'1'*64}
        for case in ['first', 'repeat', 'failed']:
            if case=='repeat':
                relocated = base/'relocated-dependencies'
                shutil.copytree(dependencies, relocated)
                dependencies = relocated
            project = base/case
            shutil.copytree(ROOT/'examples/builds/python-pip/project', project)
            for filename in ['.env', '.npmrc', 'oyzu.local.toml']:
                (project/filename).write_text('synthetic-local-credential-marker')
            (project/'.private').mkdir()
            (project/'.private/token').write_text('synthetic-local-credential-marker')
            (project/'templates').mkdir()
            (project/'templates/health.txt').write_text('visible application data')
            def run(*command, success=True):
                result = subprocess.run(list(map(str, command)), cwd=project, env=env, capture_output=True, text=True, timeout=120)
                assert (result.returncode==0)==success, (result.stdout,result.stderr)
                return result
            run(sys.executable, '-I', RUNTIME/'application.py', 'prepare', dependencies)
            run(sys.executable, '-I', RUNTIME/'application.py', 'build', dependencies)
            artifact = project/'.oyzu-build/application.pyz'
            hashes.append(hashlib.sha256(artifact.read_bytes()).hexdigest())
            # This interpreter starts without site packages; packaging must come from the archive.
            result = run(sys.executable, '-I', '-S', artifact)
            assert "'status': 'ok'" in result.stdout
            with zipfile.ZipFile(artifact) as archive:
                names = archive.namelist()
                assert 'app.py' in names and '__main__.py' in names
                assert not any(n.startswith(('tests/', 'pytest/', '_pytest/', 'setuptools/')) for n in names)
                assert not any(n.endswith('/direct_url.json') for n in names)
                assert not any(n.startswith('.') or n in {'oyzu.local.toml', 'requirements.txt'} for n in names)
                assert archive.read('templates/health.txt')==b'visible application data'
                assert all(b'synthetic-local-credential-marker' not in archive.read(n) for n in names)
                metadata = json.loads(archive.read('oyzu-application.json'))
                assert metadata['runtimePackages']==['packaging/24.2']
                assert metadata['sourceFiles']==['app.py']
            (project/'app.py').write_text('raise AssertionError("checkout imported instead of built archive")\n')
            if case=='failed':
                test = project/'tests/test_app.py'
                test.write_text(test.read_text().replace('{"status": "ok"}', '{"status": "failed"}'))
            python = project/'.oyzu-build/venv'/('Scripts/python.exe' if os.name=='nt' else 'bin/python')
            run(python, '-I', RUNTIME/'application.py', 'test', 'junit.xml', 'coverage.xml', success=case!='failed')
            report = ET.parse(project/'junit.xml')
            assert len(report.findall('.//testcase'))==2
            assert bool(report.findall('.//failure'))==(case=='failed')
            coverage = ET.parse(project/'coverage.xml').getroot()
            assert int(coverage.attrib['lines-covered'])>0
            assert [c.attrib['filename'].replace('\\','/').split('/')[-1] for c in coverage.findall('.//class')]==['app.py']
            if case=='first':
                # Quality checks run on source and exclude prepared runtime state.
                shutil.copyfile(ROOT/'examples/builds/python-pip/project/app.py', project/'app.py')
                ruff = python.with_name('ruff.exe' if os.name=='nt' else 'ruff')
                run(ruff, 'check', '.', '--no-fix')
                run(ruff, 'format', '--check', '.')
                app = project/'app.py'
                unchanged = 'import json\n'+app.read_text()
                app.write_text(unchanged)
                (project/'ruff.toml').write_text('fix = true\n')
                run(ruff, 'check', '.', '--no-fix', success=False)
                assert app.read_text()==unchanged, 'native fix configuration mutated source during lint'
                original = artifact.read_bytes()
                artifact.write_bytes(original+b'changed after tests')
                result = run(sys.executable, '-I', RUNTIME/'application.py', 'package', success=False)
                assert 'changed after testing' in result.stderr
                artifact.write_bytes(original)
        assert len(set(hashes))==1, 'snapshot archive changed across identical input copies'
    print('Python requirements app: native pip, repeatable executable archive, runtime-only dependencies, artifact-source tests/coverage and failure evidence passed')


if __name__=='__main__':
    main()
