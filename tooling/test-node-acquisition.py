"""Native npm adapter probe; sandbox and Rust broker enforcement are CI build checks.

The fixture transport serves one pinned upstream tarball through the production
spool protocol. npm itself runs offline against fresh private caches.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
URL = 'https://registry.npmjs.org/is-number/-/is-number-7.0.0.tgz'


def contents(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file()}


def main():
    with urllib.request.urlopen(URL, timeout=30) as response:
        tarball = response.read()
    with tempfile.TemporaryDirectory(prefix='oyzu-npm-probe-') as temporary:
        base = Path(temporary)
        runtime = base / 'runtime'
        runtime.mkdir()
        for src, name in [
            ('src/builders/node/runtime/npm.mjs', 'npm.mjs'),
            ('src/builders/node/runtime/lock.mjs', 'npm_lock.mjs'),
            ('src/broker/runtime/transport.mjs', 'broker_transport.mjs'),
        ]:
            shutil.copyfile(ROOT / src, runtime / name)
        spool = base / 'broker'
        spool.mkdir()
        stop = threading.Event()
        requests = []
        failures = []

        def serve():
            try:
                while not stop.is_set():
                    for request in spool.glob('*.request'):
                        url = json.loads(request.read_text())['url']
                        requests.append(url)
                        request.unlink()
                        accepted = url == URL
                        request.with_suffix('.body').write_bytes(tarball if accepted else b'denied')
                        metadata = request.with_suffix('.pending-response')
                        metadata.write_text(json.dumps({'status': 200 if accepted else 403, 'sourceId': 'npm-public'}))
                        metadata.rename(request.with_suffix('.response'))
                    time.sleep(.01)
            except Exception as error:
                failures.append(error)

        thread = threading.Thread(target=serve)
        thread.start()
        try:
            def execute(mode, output, project, success=True):
                result = subprocess.run(['node', str(runtime / 'npm.mjs'), mode, str(output), str(project), str(spool)],
                                        capture_output=True, text=True, timeout=180)
                assert (result.returncode == 0) == success, result.stdout + result.stderr
                return result

            captures = []
            for index in range(2):
                project = base / f'project {index}'
                shutil.copytree(ROOT / 'tooling/fixtures/npm-registry', project)
                # Capture must not run repository lifecycle code; execution must.
                package = json.loads((project / 'package.json').read_text())
                package['scripts']['preinstall'] = 'node lifecycle.cjs'
                (project / 'package.json').write_text(json.dumps(package))
                (project / 'lifecycle.cjs').write_text("require('node:fs').writeFileSync('lifecycle-ran','yes');\n")
                output = base / f'capture {index}'
                output.mkdir()
                execute('acquire', output, project)
                assert not (project / 'lifecycle-ran').exists()
                captures.append(contents(output))
                shutil.rmtree(project / 'node_modules')
                count = len(requests)
                execute('install', output, project)
                assert (project / 'lifecycle-ran').read_text() == 'yes'
                assert len(requests) == count, 'execution contacted acquisition broker'
                assert contents(output) == captures[-1], 'execution mutated captured inputs'
                result = subprocess.run(['node', '--test'], cwd=project, capture_output=True, text=True)
                assert result.returncode == 0, result.stdout + result.stderr
            assert captures[0] == captures[1], 'capture depends on cache timestamp or absolute location'
            lock_path = project / 'package-lock.json'
            lock = json.loads(lock_path.read_text())
            entry = lock['packages']['node_modules/is-number']
            entry['integrity'] = 'sha512-' + 'A' * 86 + '=='
            lock_path.write_text(json.dumps(lock))
            bad = base / 'bad-integrity'
            bad.mkdir()
            assert 'integrity mismatch' in execute('acquire', bad, project, False).stderr
            entry['integrity'] = json.loads((ROOT / 'tooling/fixtures/npm-registry/package-lock.json').read_text())['packages']['node_modules/is-number']['integrity']
            entry['resolved'] = 'https://unapproved.invalid/package.tgz'
            lock_path.write_text(json.dumps(lock))
            denied = base / 'denied-source'
            denied.mkdir()
            assert 'denied or failed' in execute('acquire', denied, project, False).stderr
            assert not failures, failures
        finally:
            stop.set()
            thread.join()
    print('Native npm capture/replay passed: deterministic tarballs, offline installation, lifecycle isolation, integrity and source denial')


if __name__ == '__main__':
    main()
