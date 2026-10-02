"""Native backend wheel -> runnable app -> archive-source tests and evidence."""
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--wheels', type=Path, required=True)
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location('adapter', RUNTIME/'adapter.py')
    adapter = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(adapter)
    with tempfile.TemporaryDirectory(prefix='oyzu wheel application ') as temporary:
        base = Path(temporary)
        dependencies = base/'dependencies'
        shutil.copytree(args.wheels, dependencies/'wheels')
        adapter.inventory({'packaging':'runtime'}, ['packaging==24.2'], dependencies)
        version = '0.1.0.dev0+g0123456789ab'
        env = {**os.environ, 'OYZU_VERSION':version, 'OYZU_TARGET':'api',
               # Native Windows pip creates ZIP-based launchers (1980 minimum).
               'OYZU_SOURCE_DIGEST':'sha256:'+'1'*64, 'SOURCE_DATE_EPOCH':'315532800',
               'OYZU_PYTHON_DISTRIBUTION':'oyzu_api_example',
               'OYZU_PYTHON_ENTRYPOINT_NAME':'oyzu-example-api',
               'OYZU_PYTHON_ENTRYPOINT_VALUE':'api:main'}
        hashes = []
        for case in ['first', 'repeat', 'failed']:
            project = base/case
            shutil.copytree(ROOT/'examples/builds/python-api/project', project)
            metadata = project/'pyproject.toml'
            metadata.write_text(metadata.read_text().replace('version = "0.1.0"', f'version = "{version}"').replace('dependencies = []', 'dependencies = ["packaging==24.2"]').replace('packages = ["api"]', 'packages = ["api", "bin"]'))
            (project/'bin').mkdir()
            (project/'bin/__init__.py').write_text('VALUE = "1.0"\n')
            (project/'api/__init__.py').write_text('''from packaging.version import Version
from bin import VALUE


def value():
    return str(Version(VALUE))


def main():
    print("application " + value())
''')
            (project/'tests/test_api.py').write_text('from api import value\n\ndef test_value():\n    assert value() == "1.0"\n')
            # The native backend owns inclusion; visible unrelated checkout files
            # must not be copied into this distribution-based application.
            (project/'private.txt').write_text('not-in-the-wheel')
            def run(*command, success=True, overrides=None):
                result = subprocess.run(list(map(str, command)), cwd=project,
                                        env={**env, **(overrides or {})}, capture_output=True, text=True, timeout=120)
                assert (result.returncode == 0) == success, (result.stdout, result.stderr)
                return result
            run(sys.executable, '-I', RUNTIME/'application.py', 'prepare', dependencies)
            python = project/'.oyzu-build/venv'/('Scripts/python.exe' if os.name == 'nt' else 'bin/python')
            run(python, '-I', '-m', 'build', '--no-isolation', '--outdir', '.oyzu-build/dist')
            wheel = next((project/'.oyzu-build/dist').glob('*.whl'))
            original = wheel.read_bytes()
            if case == 'first':
                result = run(sys.executable, '-I', RUNTIME/'distribution_app.py', 'assemble', dependencies,
                             success=False, overrides={'OYZU_PYTHON_ENTRYPOINT_VALUE':'api:wrong'})
                assert 'entrypoint differs from plan' in result.stderr
                shutil.rmtree(project/'.oyzu-build/application-project')
            run(sys.executable, '-I', RUNTIME/'distribution_app.py', 'assemble', dependencies)
            assert wheel.read_bytes() == original
            artifact = project/'.oyzu-build/application.pyz'
            hashes.append(hashlib.sha256(artifact.read_bytes()).hexdigest())
            # No installed packages: both project and packaging must be in the zip.
            assert run(sys.executable, '-I', '-S', artifact).stdout.strip() == 'application 1.0'
            with zipfile.ZipFile(artifact) as archive:
                names = archive.namelist()
                facts = json.loads(archive.read('oyzu-application.json'))
                assert facts['entrypoint'] == {'name':'oyzu-example-api', 'value':'api:main'}
                assert facts['sourceFiles'] == ['api/__init__.py', 'bin/__init__.py']
                assert facts['runtimePackages'] == ['packaging/24.2']
                assert 'bin/__init__.py' in names
                assert not any(n.startswith(('Scripts/', '_pytest/', 'setuptools/', 'tests/')) for n in names)
                assert 'bin/oyzu-example-api' not in names
                assert not any(n.endswith('/direct_url.json') for n in names)
                assert 'private.txt' not in names
            (project/'api/__init__.py').write_text('raise AssertionError("checkout must not be imported")\n')
            if case == 'failed':
                (project/'tests/test_api.py').write_text('from api import value\n\ndef test_failure():\n    assert value() == "wrong"\n')
            run(python, '-I', RUNTIME/'application.py', 'test', 'junit.xml', 'coverage.xml', success=case != 'failed')
            report = ET.parse(project/'junit.xml')
            assert len(report.findall('.//testcase')) == 1
            assert bool(report.findall('.//failure')) == (case == 'failed')
            coverage = ET.parse(project/'coverage.xml').getroot()
            assert int(coverage.attrib['lines-valid']) > 0
            if case != 'failed':
                assert int(coverage.attrib['lines-covered']) > 0
            if case == 'first':
                wheel.write_bytes(original+b'changed')
                result = run(sys.executable, '-I', RUNTIME/'distribution_app.py', 'package', success=False)
                assert 'artifacts changed after assembly' in result.stderr
                wheel.write_bytes(original)
                artifact.write_bytes(artifact.read_bytes()+b'changed')
                result = run(sys.executable, '-I', RUNTIME/'distribution_app.py', 'package', success=False)
                assert 'changed after testing' in result.stderr
        assert len(set(hashes)) == 1, 'Application artifact changes across identical relocated builds'
    print('Python distribution app: native wheel, executable entrypoint, runtime closure, archive-source reports, identity mismatch, failure and tampering checks passed')


if __name__ == '__main__':
    main()
