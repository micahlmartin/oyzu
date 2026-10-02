"""Native setuptools metadata/version projection; C compilation requires Linux."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
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
    with tempfile.TemporaryDirectory(prefix='oyzu legacy ') as temporary:
        base = Path(temporary)
        dependencies = base/'dependencies'
        shutil.copytree(args.wheels, dependencies/'wheels')
        env = {**os.environ, 'OYZU_SOURCE_DIGEST':'sha256:'+'1'*64, 'SOURCE_DATE_EPOCH':'0', 'PYTHONHASHSEED':'0'}
        for case in ['pure', *(['native'] if sys.platform=='linux' else [])]:
            project = base/case
            if case=='native':
                shutil.copytree(ROOT/'examples/builds/python-legacy-native/project', project)
            else:
                project.mkdir()
                (project/'setup.py').write_text('from setuptools import setup\nsetup(name="legacy-demo", version="1.2.3", py_modules=["demo"])\n')
                (project/'demo.py').write_text('def add(a, b):\n    return a+b\n')
            old = Path.cwd()
            try:
                os.chdir(project)
                adapter.prepare_environment(dependencies)
            finally:
                os.chdir(old)
            python = project/'.oyzu-build/venv'/('Scripts/python.exe' if os.name=='nt' else 'bin/python')
            def run(*argv, success=True):
                result = subprocess.run(list(map(str, argv)), cwd=project, env=env, capture_output=True, text=True, timeout=120)
                assert (result.returncode==0)==success, (result.stdout,result.stderr)
                return result
            metadata = base/(case+'.json')
            run(python, '-I', RUNTIME/'legacy.py', 'metadata', metadata)
            first = metadata.read_bytes()
            run(python, '-I', RUNTIME/'legacy.py', 'metadata', metadata)
            assert metadata.read_bytes()==first
            model = json.loads(first)
            assert model['version'].endswith('.dev0+g111111111111')
            assert bool(model['compiler'])==(case=='native')
            assert bool(model['extensions'])==(case=='native')
            env['OYZU_VERSION'] = model['version']
            run(python, '-I', RUNTIME/'legacy.py', 'build', metadata)
            wheel, = (project/'.oyzu-build/dist').glob('*.whl')
            with zipfile.ZipFile(wheel) as archive:
                meta = next(n for n in archive.namelist() if n.endswith('.dist-info/METADATA'))
                assert 'Version: '+model['version'] in archive.read(meta).decode()
                if case=='native':
                    assert any(n.startswith('native_math.') and n.endswith('.so') for n in archive.namelist())
            run(sys.executable, '-I', '-m', 'pip', '--isolated', '--python', python, 'install', '--no-index', '--no-deps', wheel)
            module = 'native_math' if case=='native' else 'demo'
            run(python, '-I', '-c', f'import {module}; assert {module}.add(2, 3)==5')
            # The native sdist must retain the snapshot tag when rebuilt without
            # Oyzu's version arguments, not just advertise it in PKG-INFO.
            source_archive, = (project/'.oyzu-build/dist').glob('*.tar.gz')
            extracted = base/(case+'-sdist')
            extracted.mkdir()
            with tarfile.open(source_archive) as archive:
                archive.extractall(extracted, filter='data')
            source, = extracted.iterdir()
            rebuilt = base/(case+'-rebuilt')
            run(python, '-I', '-m', 'build', '--wheel', '--no-isolation', '--outdir', rebuilt, source)
            assert (rebuilt/wheel.name).is_file(), 'sdist rebuild lost the snapshot version'
            if case=='native':
                env['CC'] = 'oyzu-missing-compiler'
                result = run(python, '-I', RUNTIME/'legacy.py', 'metadata', metadata, success=False)
                assert 'compiler is unavailable' in result.stderr
                del env['CC']
    print('Native setuptools metadata, snapshot version, exact wheel/sdist names and installed artifact execution passed; C extension '+('verified' if sys.platform=='linux' else 'requires Linux CI'))


if __name__=='__main__':
    main()
