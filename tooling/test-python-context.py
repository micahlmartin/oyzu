"""Native lock export and offline hash-checked installation with provisioned tools.

Fixture lock creation/download uses the network. Export and installation are
offline; this is native protocol verification, not broker/isolation acceptance.
"""
import argparse
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

from python_context_fixtures import create, lock_command

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('oyzu_python', ROOT / 'src/builders/python/runtime/adapter.py')
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


def run(argv, *, success=True):
    result = subprocess.run(argv, capture_output=True, text=True, timeout=300)
    assert (result.returncode == 0) == success, (argv, result.stdout, result.stderr)
    return result


def verify(manager):
    with tempfile.TemporaryDirectory(prefix='oyzu-python-context-') as temporary, patch.dict(os.environ, {
        'POETRY_CACHE_DIR': str(Path(temporary) / 'cache' / 'poetry'),
        'POETRY_CONFIG_DIR': str(Path(temporary) / 'config' / 'poetry'),
        'POETRY_DATA_DIR': str(Path(temporary) / 'data' / 'poetry'),
        'POETRY_VIRTUALENVS_IN_PROJECT': 'false',
        'UV_CACHE_DIR': str(Path(temporary) / 'cache' / 'uv'),
    }):
        root = Path(temporary)
        project = root / 'project'
        create(project, manager)
        before = Path.cwd()
        try:
            os.chdir(project)
            run(lock_command(manager, sys.executable))
            lock = project / ('uv.lock' if manager == 'uv' else 'poetry.lock')
            original = lock.read_bytes()
            exported = root / 'export.txt'
            adapter.locked_export(manager, exported, runtime_only=True)
            assert lock.read_bytes() == original
            content = exported.read_text(encoding='utf-8')
            assert 'six==' in content and 'idna==' not in content and 'packaging==' not in content
            store = root / 'wheels'
            store.mkdir()
            run([sys.executable, '-I', '-m', 'pip', '--isolated', 'download', '--no-deps', '--only-binary=:all:', '--require-hashes', '--dest', str(store), '-r', str(exported)])
            assert len(list(store.glob('*.whl'))) == 1
            adapter.write_install_manifest(root)
            manifest = store / 'requirements.txt'
            assert '--hash=sha256:' in manifest.read_text()
            site = root / 'installed'
            install = [sys.executable, '-I', '-m', 'pip', '--isolated', 'install', '--no-index', '--no-deps', '--no-compile', '--require-hashes', '--find-links', str(store), '-r', str(manifest)]
            run([*install, '--target', str(site)])
            run([sys.executable, '-I', '-B', '-S', '-c', "import sys; sys.path.insert(0,sys.argv[1]); import six; assert six.__version__=='1.17.0'; assert six.__file__.startswith(sys.argv[1])", str(site)])
            wheel, = store.glob('*.whl')
            wheel.write_bytes(wheel.read_bytes() + b'tampered')
            rejected = run([*install, '--target', str(root / 'rejected')], success=False)
            assert 'hash' in rejected.stderr.lower()
            (project / 'pyproject.toml').write_text((project / 'pyproject.toml').read_text().replace('six==1.17.0', 'six==1.16.0'))
            try:
                adapter.locked_export(manager, root / 'stale.txt', runtime_only=True)
            except (ValueError, subprocess.CalledProcessError):
                pass
            else:
                raise AssertionError('stale native lock was accepted')
            assert lock.read_bytes() == original
        finally:
            os.chdir(before)
    print(f'{manager}: native runtime-only export, unchanged lock, offline hashed install/import, tamper and stale-lock rejection passed', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--manager', required=True, choices=['uv', 'poetry'])
    verify(parser.parse_args().manager)
