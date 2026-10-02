"""Real pnpm capture/replay with a fixture broker; worker isolation is tested in CI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
URLS = [f'https://registry.npmjs.org/{name}/-/{name}-{version}.tgz'
        for name, version in [('is-odd', '3.0.1'), ('is-number', '6.0.0')]]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--pnpm-cli', type=Path, required=True)
    args = parser.parse_args()
    native = args.pnpm_cli.resolve()
    env = dict(os.environ, OYZU_PNPM_YAML=str(native.parents[2] / 'yaml'))
    bodies = {}
    for url in URLS:
        with urllib.request.urlopen(url, timeout=30) as response:
            bodies[url] = response.read()
    with tempfile.TemporaryDirectory(prefix='oyzu pnpm capture ') as temporary:
        base = Path(temporary)
        runtime = base / 'runtime'
        shutil.copytree(ROOT / 'src/builders/node/runtime', runtime)
        shutil.copyfile(ROOT / 'src/broker/runtime/transport.mjs', runtime / 'broker_transport.mjs')
        wrapper = base / 'run.mjs'
        wrapper.write_text(f"import {{profile}} from {json.dumps((runtime/'pnpm.mjs').as_uri())};\n"
                           f"import {{run}} from {json.dumps((runtime/'manager-runtime.mjs').as_uri())};\n"
                           f"profile.command.splice(0,1,process.execPath,{json.dumps(str(native))}); await run(profile);\n")
        spool = base / 'broker'
        spool.mkdir()
        requests, failures = [], []
        stop = threading.Event()

        def serve():
            try:
                while not stop.is_set():
                    for request in spool.glob('*.request'):
                        url = json.loads(request.read_text())['url']
                        requests.append(url)
                        request.unlink()
                        request.with_suffix('.body').write_bytes(bodies.get(url, b'denied'))
                        response = request.with_suffix('.pending-response')
                        response.write_text(json.dumps({'status': 200 if url in bodies else 403, 'sourceId': 'npm-public'}))
                        response.rename(request.with_suffix('.response'))
                    time.sleep(.01)
            except Exception as error:
                failures.append(error)

        thread = threading.Thread(target=serve)
        thread.start()
        try:
            def run(mode, output, project, success=True):
                result = subprocess.run(['node', str(wrapper), mode, str(output), str(project), str(spool)],
                                        env=env, capture_output=True, text=True, encoding='utf-8', timeout=150)
                assert (result.returncode == 0) == success, result.stdout + result.stderr
                return result

            def tree(path):
                return {p.relative_to(path).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                        for p in path.rglob('*') if p.is_file()}

            captures = []
            for index in range(2):
                project = base / f'project {index}'
                shutil.copytree(ROOT / 'tooling/fixtures/pnpm-registry', project)
                package_path = project / 'package.json'
                package = json.loads(package_path.read_text())
                package['scripts']['preinstall'] = 'node lifecycle.cjs'
                package_path.write_text(json.dumps(package))
                (project / 'lifecycle.cjs').write_text("require('node:fs').writeFileSync('lifecycle-ran', 'yes');\n")
                output = base / f'capture {index}'
                output.mkdir()
                original_lock = (project / 'pnpm-lock.yaml').read_bytes()
                run('acquire', output, project)
                assert not (project / 'lifecycle-ran').exists()
                inventory = json.loads((output / 'inventory.json').read_text())
                assert {(p['name'], p['version']) for p in inventory['packages']} == {('is-odd', '3.0.1'), ('is-number', '6.0.0')}
                assert all(p['sourceId'] == 'npm-public' for p in inventory['packages'])
                captures.append(tree(output))
                shutil.rmtree(project / 'node_modules')
                count = len(requests)
                run('install', output, project)
                assert len(requests) == count and tree(output) == captures[-1]
                assert (project / 'lifecycle-ran').read_text() == 'yes'
                result = subprocess.run(['node', '--test'], cwd=project, capture_output=True, text=True)
                assert result.returncode == 0, result.stdout + result.stderr
                assert (project / 'pnpm-lock.yaml').read_bytes() == original_lock
            assert captures[0] == captures[1], 'captured bytes depend on location/time'
            assert sorted(requests) == sorted(URLS * 2)
            # Failures must not silently resolve a different version or execute source hooks.
            shutil.rmtree(project / 'node_modules')
            archive = next((output / 'tarballs').glob('*.tgz'))
            original = archive.read_bytes()
            archive.write_bytes(b'corrupt')
            assert 'integrity mismatch' in run('install', output, project, False).stderr
            archive.write_bytes(original)
            package['dependencies']['is-odd'] = '2.0.0'
            package_path.write_text(json.dumps(package))
            assert 'OUTDATED_LOCKFILE' in run('install', output, project, False).stderr
            package['dependencies']['is-odd'] = '3.0.1'
            package_path.write_text(json.dumps(package))
            lock_path = project / 'pnpm-lock.yaml'
            lock_path.write_bytes(original_lock.replace(b'version: 3.0.1', b'version: link:/outside'))
            assert 'non-registry dependency references' in run('acquire', output, project, False).stderr
            lock_path.write_bytes(original_lock)
            (project / '.pnpmfile.cjs').write_text("throw new Error('project hook executed');")
            assert 'does not yet support .pnpmfile.cjs' in run('acquire', output, project, False).stderr
            assert len(requests) == count
            assert not failures, failures
            print('pnpm: native transitive capture/replay, lifecycle isolation, frozen lock, digest rejection and deterministic inputs passed')
        finally:
            stop.set()
            thread.join()


if __name__ == '__main__':
    main()
