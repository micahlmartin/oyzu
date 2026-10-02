"""Real native manager capture/replay; worker isolation is tested separately in CI."""
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--manager', choices=['pnpm', 'yarn'], required=True)
    parser.add_argument('--native-cli', type=Path, required=True)
    parser.add_argument('--resolutions', action='store_true', help='Exercise the Yarn selective-resolution fixture')
    args = parser.parse_args()
    manager = args.manager
    if args.resolutions and manager != 'yarn':
        parser.error('--resolutions requires --manager yarn')
    native = args.native_cli.resolve()
    env = dict(os.environ, OYZU_PNPM_YAML=str(native.parents[2] / 'yaml'),
               OYZU_YARN_LOCKFILE=str(native.parents[2] / '@yarnpkg/lockfile'))
    expected = {('is-odd', '3.0.1'), ('is-number', '6.0.0')}
    if manager == 'yarn':
        expected.add(('@colors/colors', '1.6.0'))
    if args.resolutions:
        expected.add(('is-number', '7.0.0'))
    fixture = ('examples/builds/node-managers/variants/yarn-resolutions' if args.resolutions
               else f'tooling/fixtures/{manager}-registry')
    urls = [f'https://registry.npmjs.org/{name}/-/{name.split("/")[-1]}-{version}.tgz'
            for name, version in sorted(expected)]
    bodies = {}
    for url in urls:
        with urllib.request.urlopen(url, timeout=30) as response:
            bodies[url] = response.read()
    with tempfile.TemporaryDirectory(prefix=f'oyzu {manager} capture ') as temporary:
        base = Path(temporary)
        runtime = base / 'runtime'
        shutil.copytree(ROOT / 'src/builders/node/runtime', runtime)
        shutil.copyfile(ROOT / 'src/broker/runtime/transport.mjs', runtime / 'broker_transport.mjs')
        wrapper = base / 'run.mjs'
        wrapper.write_text(f"import {{profile}} from {json.dumps((runtime/f'{manager}.mjs').as_uri())};\n"
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
                shutil.copytree(ROOT / fixture, project)
                package_path = project / 'package.json'
                package = json.loads(package_path.read_text())
                package['scripts']['preinstall'] = 'node lifecycle.cjs'
                package_path.write_text(json.dumps(package))
                (project / 'lifecycle.cjs').write_text("require('node:fs').writeFileSync('lifecycle-ran', 'yes');\n")
                output = base / f'capture {index}'
                output.mkdir()
                lock_path = project / ('pnpm-lock.yaml' if manager == 'pnpm' else 'yarn.lock')
                original_lock = lock_path.read_bytes()
                run('acquire', output, project)
                assert not (project / 'lifecycle-ran').exists()
                inventory = json.loads((output / 'inventory.json').read_text())
                assert {(p['name'], p['version']) for p in inventory['packages']} == expected
                assert all(p['sourceId'] == 'npm-public' for p in inventory['packages'])
                captures.append(tree(output))
                shutil.rmtree(project / 'node_modules')
                count = len(requests)
                run('install', output, project)
                assert len(requests) == count and tree(output) == captures[-1]
                assert (project / 'lifecycle-ran').read_text() == 'yes'
                result = subprocess.run(['node', '--test'], cwd=project, capture_output=True, text=True)
                assert result.returncode == 0, result.stdout + result.stderr
                assert lock_path.read_bytes() == original_lock
            assert captures[0] == captures[1], 'captured bytes depend on location/time'
            assert sorted(requests) == sorted(urls * 2)
            if args.resolutions:
                # Both versions exist in the mirror. Native resolution must still
                # reject the changed graph before running any project hooks.
                shutil.rmtree(project / 'node_modules')
                (project / 'lifecycle-ran').unlink()
                package['resolutions']['is-odd/is-number'] = '6.0.0'
                package_path.write_text(json.dumps(package))
                assert 'native dependency resolution changed' in run('install', output, project, False).stderr
                assert not (project / 'lifecycle-ran').exists()
                assert len(requests) == count and tree(output) == captures[-1]
                lock_path.write_bytes(original_lock)
                package['resolutions']['is-odd/is-number'] = '7.0.0'
                package_path.write_text(json.dumps(package))
                run('install', output, project)
            # Failures must not silently resolve a different version or execute source hooks.
            shutil.rmtree(project / 'node_modules')
            archive = next((output / 'tarballs').glob('*.tgz'))
            original = archive.read_bytes()
            archive.write_bytes(b'corrupt')
            assert 'integrity mismatch' in run('install', output, project, False).stderr
            archive.write_bytes(original)
            package['dependencies']['is-odd'] = '2.0.0'
            package_path.write_text(json.dumps(package))
            stale = run('install', output, project, False).stderr
            assert ('OUTDATED_LOCKFILE' if manager == 'pnpm' else 'current frozen lockfile') in stale
            package['dependencies']['is-odd'] = '3.0.1'
            package_path.write_text(json.dumps(package))
            if manager == 'pnpm':
                lock_path.write_bytes(original_lock.replace(b'version: 3.0.1', b'version: link:/outside'))
                assert 'non-registry dependency references' in run('acquire', output, project, False).stderr
                lock_path.write_bytes(original_lock)
                (project / '.pnpmfile.cjs').write_text("throw new Error('project hook executed');")
                assert 'does not yet support .pnpmfile.cjs' in run('acquire', output, project, False).stderr
            else:
                package['resolutions'] = {'is-number': 'file:../outside'}
                package_path.write_text(json.dumps(package))
                assert 'registry version ranges' in run('acquire', output, project, False).stderr
                package.pop('resolutions')
                if args.resolutions:
                    package['resolutions'] = {'is-odd/is-number': '7.0.0'}
                package_path.write_text(json.dumps(package))
                lock_path.write_bytes(original_lock.replace(b'https://registry.npmjs.org/is-odd/-/is-odd-3.0.1.tgz', b'file:/outside'))
                assert 'credential-free registry archive' in run('acquire', output, project, False).stderr
                lock_path.write_bytes(original_lock)
                (project / '.yarnrc').write_text('yarn-path "outside.js"\n')
                assert 'does not yet support .yarnrc' in run('acquire', output, project, False).stderr
            assert len(requests) == count
            assert not failures, failures
            print(f'{manager}: native transitive capture/replay, lifecycle isolation, frozen lock, digest rejection and deterministic inputs passed (resolutions={args.resolutions})')
        finally:
            stop.set()
            thread.join()


if __name__ == '__main__':
    main()
