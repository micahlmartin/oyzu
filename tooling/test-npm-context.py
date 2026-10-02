"""Native npm acquisition -> tarballs only -> fresh offline cache/install.

The spool is a bounded fixture transport, not evidence of container isolation.
The captured Docker suite verifies the real broker and network boundary.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import urllib.request

from npm_context_fixtures import create, lock_arguments

ROOT = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix='oyzu npm context ') as directory:
        base = Path(directory)
        project = base / 'project'
        create(project)
        runtime = base / 'runtime'
        shutil.copytree(ROOT / 'src/builders/node/runtime', runtime)
        shutil.copyfile(runtime / 'lock.mjs', runtime / 'npm_lock.mjs')
        shutil.copyfile(ROOT / 'src/broker/runtime/transport.mjs', runtime / 'broker_transport.mjs')
        # Use the product's same native entrypoint binding on Windows/Linux.
        wrapper = base / 'npm.mjs'
        wrapper.write_text(
            f'import {{npmCommand}} from {json.dumps((runtime / "npm-native.mjs").as_uri())};\n'
            'import {spawnSync} from "node:child_process";\n'
            'const [cache,...args] = process.argv.slice(2);\n'
            'const [command,...argv] = npmCommand(args,cache);\n'
            'const result = spawnSync(command,argv,{stdio:"inherit"});\n'
            'if(result.error) throw result.error; process.exit(result.status ?? 1);\n', encoding='utf-8')
        env = dict(os.environ, HOME=str(base / 'home'), USERPROFILE=str(base / 'home'))

        def run(argv, cwd=project, success=True):
            result = subprocess.run(argv, cwd=cwd, env=env, capture_output=True,
                                    text=True, encoding='utf-8', timeout=240)
            assert (result.returncode == 0) == success, (result.stdout, result.stderr)
            return result

        def npm(args, cache, cwd=project, success=True):
            return run(['node', str(wrapper), str(cache), *args], cwd, success)

        # Fixture setup deliberately enables the registry once to get a native
        # lock. Captured and replay operations below do not use that cache.
        npm([*lock_arguments(base / 'fixture-cache'), '--offline=false'], base / 'fixture-cache')
        original = {name: (project / name).read_bytes() for name in ['package.json', 'package-lock.json']}
        lock = json.loads(original['package-lock.json'])
        bodies = {}
        for package in lock['packages'].values():
            if 'resolved' in package:
                url = package['resolved']
                assert url.startswith('https://registry.npmjs.org/')
                with urllib.request.urlopen(url, timeout=30) as response:
                    bodies[url] = response.read()
        spool = base / 'broker'
        spool.mkdir()
        stop = threading.Event()
        requests, failures = [], []

        def serve():
            try:
                while not stop.wait(.01):
                    for request in spool.glob('*.request'):
                        url = json.loads(request.read_text())['url']
                        requests.append(url)
                        request.unlink()
                        request.with_suffix('.body').write_bytes(bodies.get(url, b'denied'))
                        pending = request.with_suffix('.pending')
                        pending.write_text(json.dumps({'status': 200 if url in bodies else 403, 'sourceId': 'npm-public'}))
                        pending.rename(request.with_suffix('.response'))
            except Exception as error:
                failures.append(error)

        thread = threading.Thread(target=serve)
        thread.start()
        try:
            output = base / 'capture'
            output.mkdir()
            run(['node', str(runtime / 'npm.mjs'), 'acquire', str(output), str(project), str(spool)])
            inventory = json.loads((output / 'inventory.json').read_text())
            assert {(p['name'], p['version']) for p in inventory['packages']} == {
                ('is-odd', '3.0.1'), ('is-number', '6.0.0'), ('picocolors', '1.1.1')}
            assert sorted(requests) == sorted(bodies)
            assert not failures
        finally:
            stop.set()
            thread.join()
        # A different source copy receives only the exportable subtree, never
        # the capture's node_modules, cache, inventory, broker or credentials.
        store = base / 'dependencies'
        shutil.copytree(output / 'tarballs', store)
        before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in store.iterdir()}
        assert all(name == digest + '.tgz' for name, digest in before.items())
        assert len(before) == 3
        for index in range(2):
            consumer = base / f'consumer {index}'
            consumer.mkdir()
            for name, body in original.items():
                (consumer / name).write_bytes(body)
            cache = base / f'empty-cache {index}'
            for tarball in sorted(store.glob('*.tgz')):
                npm(['cache', 'add', str(tarball), '--ignore-scripts'], cache, consumer)
            npm(['ci', '--ignore-scripts', '--omit=dev'], cache, consumer)
            run(['node', '-e', "if (!require('is-odd')(3)) process.exit(1)"], consumer)
            assert not (consumer / 'node_modules/picocolors').exists()
            assert all((consumer / name).read_bytes() == body for name, body in original.items())
        assert {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in store.iterdir()} == before
        assert all((project / name).read_bytes() == body for name, body in original.items())
        # Integrity must fail in product acquisition before any install actions.
        lock['packages']['node_modules/is-odd']['integrity'] = 'sha512-' + 'A' * 86 + '=='
        (project / 'package-lock.json').write_text(json.dumps(lock))
        rejected = base / 'rejected'
        rejected.mkdir()
        stop.clear()
        thread = threading.Thread(target=serve)
        thread.start()
        try:
            result = run(['node', str(runtime / 'npm.mjs'), 'acquire', str(rejected), str(project), str(spool)], success=False)
            assert 'integrity' in result.stderr.lower()
            assert not (rejected / 'inventory.json').exists()
            assert not failures
        finally:
            stop.set()
            thread.join()
    print('Native npm context: brokered locked tarballs, disabled acquisition scripts, fresh offline installs, development omission, unchanged locks/store and rejected integrity passed')


if __name__ == '__main__':
    main()
