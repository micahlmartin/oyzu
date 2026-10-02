"""Verify exact Node admission and native engine validation without a registry."""
import json
import os
from pathlib import Path
import subprocess
import shutil
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    version = subprocess.check_output(['node', '-p', 'process.versions.node'], text=True).strip()
    with tempfile.TemporaryDirectory(prefix='oyzu-node-runtime-') as temporary:
        base = Path(temporary)
        runtime_dir = base/'runtime'
        runtime_dir.mkdir()
        for path in (ROOT/'src/builders/node/runtime').glob('*.mjs'):
            shutil.copyfile(path, runtime_dir/path.name)
        shutil.copyfile(ROOT/'src/broker/runtime/transport.mjs', runtime_dir/'broker_transport.mjs')
        shutil.copyfile(ROOT/'src/builders/node/runtime/lock.mjs', runtime_dir/'npm_lock.mjs')
        runtime = runtime_dir/'npm.mjs'
        source = base/'source'
        source.mkdir()
        package = {'name': 'runtime-probe', 'version': '1.0.0', 'engines': {'node': '>=22'}}
        (source/'package.json').write_text(json.dumps(package))

        def acquire(name, expected, success):
            output = base/name
            output.mkdir()
            result = subprocess.run(['node', str(runtime), 'acquire', str(output), str(source), str(base/'no-broker')],
                                    env={**os.environ, 'OYZU_EXPECT_NODE': expected},
                                    capture_output=True, text=True, timeout=90)
            assert (result.returncode == 0) == success, result.stdout + result.stderr
            return output, result.stderr

        output, _ = acquire('matching', version, True)
        inventory = json.loads((output/'inventory.json').read_text())
        assert inventory['nodeVersion'] == version and inventory['packages'] == []
        output, error = acquire('mismatch', '0.0.0', False)
        assert 'does not match provisioned Node' in error and not list(output.iterdir())
        package['engines']['node'] = '>=999'
        (source/'package.json').write_text(json.dumps(package))
        output, error = acquire('engines', version, False)
        assert 'EBADENGINE' in error and not list(output.iterdir())
    print(f'Node {version}: exact runtime evidence, mismatch and native engines rejection before acquisition passed')


if __name__ == '__main__':
    main()
