"""Bind native Cargo compiler artifacts to a prepared binary inventory.

Preparation supplies schemaVersion=1 and ordered binaries[{packageId, name}].
Cargo IDs are opaque and are matched without parsing. Only successful native
compiler messages can populate CARGO_TARGET_DIR/oyzu-binaries/<inventory-index>;
the shared planner owns final filenames and post-gate artifact collection.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def prepare_directory(target_root):
    # The captured planner reserves .oyzu-build/target for all native outputs.
    # Cargo's default source selection omits this private dot-directory; a
    # custom target directory elsewhere is not automatically archive-excluded.
    target_root.mkdir(parents=True, exist_ok=True)
    staging = target_root/'oyzu-binaries'
    staging.mkdir(exist_ok=True)
    if staging.is_symlink() or staging.resolve() != target_root/'oyzu-binaries':
        raise ValueError('Cargo binary staging must not be a redirected directory')
    for path in staging.iterdir():
        if path.is_symlink() or not path.is_file() or not path.name.isdecimal():
            raise ValueError('Unexpected entry in reserved Cargo binary staging')
        path.unlink()
    return staging


def build(intent, argv):
    root = Path.cwd().resolve()
    target_root = Path(os.environ.get('CARGO_TARGET_DIR', '.oyzu-build/target')).resolve()
    target_root.relative_to(root)
    if target_root == root:
        raise ValueError('Cargo target directory must be a private child directory')
    staging = prepare_directory(target_root)
    with intent.open('rb') as stream:
        data = stream.read(4 * 1024 * 1024 + 1)
    if len(data) > 4 * 1024 * 1024:
        raise ValueError('Cargo binary inventory exceeds 4 MiB')
    data = json.loads(data)
    if data.get('schemaVersion') != 1 or not isinstance(data.get('binaries'), list):
        raise ValueError('Unsupported Cargo binary inventory')
    expected = []
    for item in data['binaries']:
        key = item['packageId'], item['name']
        if not all(isinstance(value, str) and value for value in key) or key in expected:
            raise ValueError('Invalid or duplicate Cargo binary identity')
        expected.append(key)
    artifacts = {}
    finished = False
    if not argv:
        raise ValueError('Missing native Cargo command')
    child = subprocess.Popen(argv, stdout=subprocess.PIPE)
    try:
        while line := child.stdout.readline(8 * 1024 * 1024 + 1):
            if len(line) > 8 * 1024 * 1024:
                raise ValueError('Cargo message exceeds 8 MiB')
            # Keep native output in the shared action log. Proc macros can print
            # ordinary text; only Cargo's structured messages establish outputs.
            sys.stdout.buffer.write(line)
            sys.stdout.buffer.flush()
            try:
                event = json.loads(line)
            except (ValueError, UnicodeDecodeError):
                continue
            if not isinstance(event, dict):
                continue
            if event.get('reason') == 'build-finished':
                finished = event.get('success') is True
            if event.get('reason') != 'compiler-artifact':
                continue
            target = event.get('target', {})
            if 'bin' not in target.get('kind', []) or event.get('profile', {}).get('test') is not False:
                continue
            key = event.get('package_id'), target.get('name')
            if key not in expected:
                raise ValueError(f'Cargo produced an unplanned binary: {key}')
            path = event.get('executable')
            if not isinstance(path, str) or not path:
                raise ValueError(f'Cargo did not report an executable for {key}')
            if key in artifacts and artifacts[key] != path:
                raise ValueError(f'Cargo reported conflicting outputs for {key}')
            artifacts[key] = path
        status = child.wait()
    except BaseException:
        child.kill()
        child.wait()
        raise
    finally:
        child.stdout.close()
    if status != 0:
        return status if status > 0 else 128-status
    if not finished or set(artifacts) != set(expected):
        raise ValueError('Cargo did not successfully produce every planned binary')
    sources = []
    for key in expected:
        path = Path(artifacts[key])
        resolved = path.resolve(strict=True)
        resolved.relative_to(target_root)
        if path.is_symlink() or not resolved.is_file():
            raise ValueError('Cargo executable must be a contained regular file')
        sources.append(resolved)
    for index, path in enumerate(sources):
        shutil.copy2(path, staging/str(index))
    return 0


if __name__ == '__main__':
    try:
        if len(sys.argv) < 3:
            raise ValueError('usage: rust-build.py BINARY_INVENTORY CARGO_COMMAND...')
        code = build(Path(sys.argv[1]), sys.argv[2:])
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'Cargo artifact capture failed: {error}', file=sys.stderr)
        code = 1
    raise SystemExit(code)
