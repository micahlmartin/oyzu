"""Replay real Node with canonical registry names and policy spellings."""
import argparse
import json
import os
import sys
from contextlib import contextmanager
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--workspace', type=Path, required=True)
    parser.add_argument('--update', action='store_true', help='Also resolve an explicit canonical-name update online')
    parser.add_argument('--disposable-linux-host', action='store_true', help='Exercise administrative eligibility on a disposable root Linux host')
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    workspace = args.workspace.resolve(strict=True)
    store = workspace / 'store'
    source = workspace / '22.15.0' / 'oyzu.lock'
    with policy_file(args.disposable_linux_host) as admin, tempfile.TemporaryDirectory(prefix='alias-', dir=workspace) as temporary:
        project = Path(temporary).resolve()
        lock = source.read_bytes()
        (project / 'oyzu.lock').write_bytes(lock)

        def configure(names, allowed):
            declarations = ''.join(f'{json.dumps(name)} = "22.15.0"\n' for name in names)
            (project / 'oyzu.toml').write_text('[tools]\n' + declarations, encoding='utf-8')
            if admin:
                admin.write_text(json.dumps({'schemaVersion': 1, 'kind': 'local-admin-policy', 'profiles': {}, 'requiredCapabilities': [], 'settings': {'tools.allowed': {'locked': True, 'value': allowed}}}), encoding='utf-8')

        def run(arguments, expected=0):
            result = subprocess.run([str(cli), '-C', str(project)] + arguments,
                                    capture_output=True, text=True, timeout=300)
            assert result.returncode == expected, (arguments, result.stdout, result.stderr)
            return result.stdout.strip()

        configure(['core:node'], ['node'])
        run(['install', '--store', str(store), '--frozen', '--offline'])
        assert run(['exec', '--store', str(store), '--', 'node', '--version']) == 'v22.15.0'
        if args.update:
            run(['install', '--store', str(store), '--update', 'core:node'])
        assert (project / 'oyzu.lock').read_bytes() == lock
        configure(['node'], ['core:node'])
        assert run(['exec', '--store', str(store), '--', 'node', '--version']) == 'v22.15.0'
        configure(['node', 'core:node'], ['core:node'])
        assert 'TOOL_ALIAS_AMBIGUOUS' in run_error(cli, project, store)
        if admin:
            configure(['core:node'], ['go'])
            assert 'CONFIG_OVERRIDE_DENIED' in run_error(cli, project, store)
        assert (project / 'oyzu.lock').read_bytes() == lock
    print(json.dumps({'verified': ['canonical request reuses short-name lock', 'duplicate aliases fail', 'lock bytes preserved'] +
                     (['canonical-name explicit update'] if args.update else []) +
                     (['short/canonical administrative eligibility and denial'] if args.disposable_linux_host else [])}, indent=2))


@contextmanager
def policy_file(enabled):
    if not enabled:
        yield None
        return
    assert sys.platform == 'linux' and os.geteuid() == 0, 'requires disposable root Linux host'
    path = Path('/etc/oyzu/admin-settings.json')
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x', encoding='utf-8'):
        pass
    try:
        yield path
    finally:
        path.unlink()


def run_error(cli, project, store):
    result = subprocess.run([str(cli), '-C', str(project), 'exec', '--store', str(store), '--', 'node', '--version'],
                            capture_output=True, text=True, timeout=300)
    assert result.returncode == 2, (result.stdout, result.stderr)
    return result.stderr


if __name__ == '__main__':
    main()
