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
            ('src/builders/node/runtime/npm-native.mjs', 'npm-native.mjs'),
            ('src/builders/node/runtime/node-runtime.mjs', 'node-runtime.mjs'),
            ('src/builders/node/runtime/npm-workspaces.mjs', 'npm-workspaces.mjs'),
            ('src/builders/node/runtime/lock.mjs', 'npm_lock.mjs'),
            ('src/builders/node/runtime/integrity.mjs', 'integrity.mjs'),
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
                                        capture_output=True, text=True, encoding='utf-8', timeout=180)
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
            native = json.loads((output / 'inventory.json').read_text())
            # Native npm generates a mixed local-workspace/registry lock using
            # the already fetched tarball; no second upstream transport is used.
            workspace = base/'mixed-workspace'
            shutil.copytree(ROOT/'examples/builds/node-workspace/project', workspace)
            # Start with the existing native registry lock; npm adds the local
            # workspace records rather than our harness manufacturing lock data.
            shutil.copyfile(ROOT/'tooling/fixtures/npm-registry/package-lock.json', workspace/'package-lock.json')
            mixed_package = json.loads((ROOT/'tooling/fixtures/npm-registry/package.json').read_text())
            mixed_package['workspaces'] = ['packages/*']
            (workspace/'package.json').write_text(json.dumps(mixed_package))
            tarball_file = base/'is-number.tgz'
            tarball_file.write_bytes(tarball)
            cache = base/'lock-cache'
            cache.mkdir()
            script = """const {npm}=await import(process.argv[1]);
const [workspace,cache,tarball]=process.argv.slice(2);
npm(['cache','add',tarball,'--ignore-scripts'],workspace,cache);
npm(['install','--package-lock-only','--ignore-scripts'],workspace,cache);
"""
            generated = subprocess.run(['node','--input-type=module','-e',script,(runtime/'npm-native.mjs').as_uri(),str(workspace),str(cache),str(tarball_file)], capture_output=True, text=True, timeout=120)
            assert generated.returncode==0, (generated.stdout,generated.stderr)
            mixed = base/'mixed-capture'
            mixed.mkdir()
            execute('acquire', mixed, workspace)
            data = json.loads((mixed/'inventory.json').read_text())
            assert [p['name'] for p in data['packages']]==['is-number']
            assert len(data['workspaces']['members'])==2
            count = len(requests)
            execute('install', mixed, workspace)
            assert len(requests)==count
            result = subprocess.run(['node','-e',"if(!require('is-number')(42))process.exit(1)"],cwd=workspace/'packages/shared', capture_output=True,text=True)
            assert result.returncode==0, result.stderr
            empty = base / 'empty-project'
            empty.mkdir()
            empty_package = {'name':'runtime-contract', 'version':'1.0.0',
                             'packageManager': 'npm@' + native['version'],
                             'scripts': {'preinstall': 'node lifecycle.cjs'}}
            (empty / 'package.json').write_text(json.dumps(empty_package))
            (empty / 'lifecycle.cjs').write_text("require('node:fs').writeFileSync('lifecycle-ran','yes');\n")
            empty_output = base / 'empty-capture'
            empty_output.mkdir()
            count = len(requests)
            execute('acquire', empty_output, empty)
            assert not (empty / 'lifecycle-ran').exists()
            empty_inventory = json.loads((empty_output / 'inventory.json').read_text())
            assert empty_inventory['packages'] == [] and empty_inventory['lockfile'] is None
            assert empty_inventory['nodeVersion'] == native['nodeVersion']
            execute('install', empty_output, empty)
            assert (empty / 'lifecycle-ran').read_text() == 'yes'
            assert len(requests) == count and not (empty / 'package-lock.json').exists()
            empty_package['packageManager'] = 'npm@0.0.0'
            (empty / 'package.json').write_text(json.dumps(empty_package))
            mismatch = base / 'wrong-manager'
            mismatch.mkdir()
            assert 'does not match provisioned' in execute('acquire', mismatch, empty, False).stderr
            assert len(requests) == count
            empty_package['packageManager'] = 'npm@' + native['version']
            empty_package['engines'] = {'node':'>=999'}
            (empty / 'package.json').write_text(json.dumps(empty_package))
            (empty / '.npmrc').write_text('force=true\nengine-strict=false\n')
            wrong_engine = base / 'wrong-engine'
            wrong_engine.mkdir()
            assert 'EBADENGINE' in execute('acquire', wrong_engine, empty, False).stderr
            (empty / '.npmrc').unlink()
            del empty_package['engines']
            (empty / 'package.json').write_text(json.dumps(empty_package))
            empty_inventory['nodeVersion'] = '0.0.0'
            (empty_output / 'inventory.json').write_text(json.dumps(empty_inventory))
            assert 'differs from the captured' in execute('install', empty_output, empty, False).stderr
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
    print('Native npm capture/replay passed: deterministic inputs, offline installation, lifecycle isolation, integrity/source denial, declared manager and engine checks, runtime binding')


if __name__ == '__main__':
    main()
