"""Offline native packaging through an invocation-private local registry.

The planner supplies dependency-ordered [{name, version}] workspace packages and
the native Cargo package command. Cargo writes and verifies each archive and its
temporary index entry; only those matching bytes join a private registry copy.
Prepared dependencies are never modified. Cargo configuration is restored even
on failure; the shared collector owns final artifact admission.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


def index_path(name):
    name = name.lower()
    if len(name) < 3:
        return f'{len(name)}/{name}'
    if len(name) == 3:
        return f'3/{name[0]}/{name}'
    return f'{name[:2]}/{name[2:4]}/{name}'


def bounded_file(path, limit):
    if path.is_symlink() or not path.is_file():
        raise ValueError(f'Expected regular Cargo registry file: {path}')
    with path.open('rb') as stream:
        data = stream.read(limit + 1)
    if len(data) > limit:
        raise ValueError(f'Cargo registry file exceeds limit: {path}')
    return data


def promote(target, registry, item):
    name, version = item['name'], item['version']
    native = target/'package/tmp-registry'
    relative = index_path(name)
    index = bounded_file(native/'index'/relative, 4 * 1024 * 1024)
    entries = [json.loads(line) for line in index.splitlines() if line.strip()]
    if len(entries) != 1 or entries[0].get('name') != name or entries[0].get('vers') != version:
        raise ValueError('Native Cargo index does not match planned package')
    filename = f'{name}-{version}.crate'
    archive = bounded_file(target/'package'/filename, 128 * 1024 * 1024)
    if hashlib.sha256(archive).hexdigest() != entries[0].get('cksum'):
        raise ValueError('Native Cargo index does not match verified archive digest')
    destination = registry/'index'/relative
    previous = bounded_file(destination, 4 * 1024 * 1024) if destination.exists() else b''
    if any(json.loads(line).get('vers') == version for line in previous.splitlines() if line.strip()):
        raise ValueError('Workspace package collides with captured registry version')
    destination.parent.mkdir(parents=True, exist_ok=True)
    (registry/filename).write_bytes(archive)
    destination.write_bytes(previous.rstrip(b'\n') + (b'\n' if previous else b'') + index.rstrip(b'\n') + b'\n')


def package(source, inventory, argv):
    if not isinstance(inventory, list) or not inventory or len(inventory) > 4096:
        raise ValueError('Invalid Cargo package inventory')
    names = set()
    for item in inventory:
        name, version = item['name'], item['version']
        if (not isinstance(name, str) or not re.fullmatch(r'[A-Za-z0-9_-]{1,64}', name)
                or not isinstance(version, str) or not re.fullmatch(r'[A-Za-z0-9_.+-]{1,128}', version)
                or name.lower() in names):
            raise ValueError('Invalid or duplicate Cargo package identity')
        names.add(name.lower())
    root = Path.cwd().resolve()
    target = Path(os.environ['CARGO_TARGET_DIR']).resolve()
    target.relative_to(root)
    if target == root:
        raise ValueError('Cargo package output must be a private child directory')
    home = Path(os.environ['CARGO_HOME']).resolve()
    config = home/'config.toml'
    original = bounded_file(config, 1024 * 1024)
    # Source capture has already rejected links; check again at this boundary.
    if source.is_symlink() or any(p.is_symlink() for p in source.rglob('*')):
        raise ValueError('Captured Cargo registry contains symlinks')
    with tempfile.TemporaryDirectory(prefix='oyzu-package-', dir=home) as temporary:
        registry = Path(temporary)/'registry'
        shutil.copytree(source, registry)
        try:
            config.write_text('[source.crates-io]\nreplace-with="oyzu-package"\n'
                              f'[source.oyzu-package]\nlocal-registry={json.dumps(registry.as_posix())}\n',
                              encoding='utf-8')
            for item in inventory:
                subprocess.run([*argv, '--package', item['name']], check=True)
                promote(target, registry, item)
        finally:
            config.write_bytes(original)


if __name__ == '__main__':
    package(Path(sys.argv[1]), json.loads(sys.argv[2]), sys.argv[3:])
