"""Native pnpm/Yarn manager adapters; sandbox acceptance uses the compiled CLI in CI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--pnpm-cli', type=Path, required=True)
    parser.add_argument('--yarn-cli', type=Path, required=True)
    parser.add_argument('--tar-stream', type=Path, required=True)
    args = parser.parse_args()
    node_version = subprocess.check_output(['node', '-p', 'process.versions.node'], text=True).strip()
    os.environ['OYZU_PNPM_YAML'] = str(args.pnpm_cli.resolve().parents[2] / 'yaml')
    os.environ['OYZU_YARN_LOCKFILE'] = str(args.yarn_cli.resolve().parents[2] / '@yarnpkg/lockfile')
    with tempfile.TemporaryDirectory(prefix='oyzu-native-managers-') as temporary:
        base = Path(temporary)
        runtime = base / 'runtime'
        shutil.copytree(ROOT / 'src/builders/node/runtime', runtime)
        shutil.copyfile(ROOT / 'src/broker/runtime/transport.mjs', runtime / 'broker_transport.mjs')
        for manager, native, expected in [('pnpm',args.pnpm_cli,'10.11.0'),('yarn',args.yarn_cli,'1.22.22')]:
            native = native.resolve()
            project = base / manager
            shutil.copytree(ROOT / 'examples/builds/node-managers' / manager,project)
            package_file = project / 'package.json'
            package = json.loads(package_file.read_text())
            package['scripts']['preinstall'] = 'node lifecycle.cjs'
            package_file.write_text(json.dumps(package))
            (project / 'lifecycle.cjs').write_text("require('node:fs').writeFileSync('lifecycle-ran','yes');\n")
            wrapper = base / (manager+'.mjs')
            wrapper.write_text(f"import {{profile}} from {json.dumps((runtime / f'{manager}.mjs').as_uri())};\n"
                               f"import {{run}} from {json.dumps((runtime / 'manager-runtime.mjs').as_uri())};\n"
                               f"profile.command.splice(0,1,process.execPath,{json.dumps(str(native))}); await run(profile);\n")
            output = base / (manager+'-capture')
            output.mkdir()

            def execute(command, success=True, env=None):
                if env is None:
                    env = {**os.environ, 'OYZU_EXPECT_NODE': node_version}
                result = subprocess.run(command,cwd=project,env=env,capture_output=True,text=True,encoding='utf-8',timeout=120)
                assert (result.returncode == 0) == success, result.stdout + result.stderr
                return result

            execute(['node',str(wrapper),'acquire',str(output),str(project)])
            assert not (project / 'lifecycle-ran').exists()
            inventory = json.loads((output / 'inventory.json').read_text())
            assert inventory['version'] == expected and inventory['nodeVersion'] == node_version
            denied = base/(manager+'-runtime-denied')
            denied.mkdir()
            mismatch = execute(['node', str(wrapper), 'acquire', str(denied), str(project)], False,
                               {**os.environ, 'OYZU_EXPECT_NODE': '0.0.0'})
            assert 'does not match provisioned Node' in mismatch.stderr
            assert not list(denied.iterdir()) and not (project/'lifecycle-ran').exists()
            altered = {**inventory, 'nodeVersion': '0.0.0'}
            (output/'inventory.json').write_text(json.dumps(altered))
            mismatch = execute(['node', str(wrapper), 'install', str(output), str(project)], False)
            assert 'differs from captured preflight' in mismatch.stderr
            assert not (project/'lifecycle-ran').exists()
            (output/'inventory.json').write_text(json.dumps(inventory))
            package['version'] = '0.1.0-dev.g0123456789ab'
            package_file.write_text(json.dumps(package))
            execute(['node',str(wrapper),'install',str(output),str(project)])
            assert (project / 'lifecycle-ran').read_text() == 'yes'
            prefix = ['node',str(native)]
            if manager == 'pnpm':
                prefix += ['--config.manage-package-manager-versions=false']
            execute(prefix + ['run','build'])
            reports = base / (manager+'-reports')
            reports.mkdir()
            execute(prefix + ['run','test','--experimental-test-coverage','--test-reporter=junit',
                              '--test-reporter-destination='+str(reports/'junit.xml'),
                              '--test-reporter=lcov','--test-reporter-destination='+str(reports/'coverage.lcov')])
            assert (reports / 'junit.xml').is_file() and 'SF:' in (reports/'coverage.lcov').read_text()
            archives = []
            for index in range(2):
                # Model fresh captured workspaces and generated files with
                # different mtimes, instead of repacking the same timestamps.
                for path in project.rglob('*'):
                    os.utime(path, (1000000000 + index * 100, 1000000000 + index * 100))
                os.utime(project, (1000000000 + index * 100, 1000000000 + index * 100))
                archive = base / f'{manager}-{index}.tgz'
                execute(prefix + ['pack','--out' if manager == 'pnpm' else '--filename',str(archive)])
                if manager == 'yarn':
                    execute(['node', str(ROOT / 'src/builders/node/runtime/archive.mjs'), str(archive), str(args.tar_stream.resolve())])
                with tarfile.open(archive) as tar:
                    assert json.load(tar.extractfile('package/package.json'))['version'] == package['version']
                    assert 'package/dist/greeting.mjs' in tar.getnames()
                archives.append(hashlib.sha256(archive.read_bytes()).hexdigest())
            assert archives[0] == archives[1], 'native package bytes changed between identical runs'
            package['packageManager'] = manager+'@0.0.0'
            package_file.write_text(json.dumps(package))
            mismatch = execute(['node',str(wrapper),'acquire',str(output),str(project)],False)
            assert '0.0.0' in mismatch.stderr, mismatch.stderr
            package['packageManager'] = manager+'@'+expected
            package['engines']['node'] = '>=999'
            package_file.write_text(json.dumps(package))
            execute(['node',str(wrapper),'acquire',str(output),str(project)],False)
            print(manager+': exact Node admission/replay identity, native manager version, frozen install, lifecycle isolation, tests/reports and reproducible snapshot package passed')


if __name__ == '__main__':
    main()
