"""Real Maven reactor -> compiled CLI tests -> module report bundles."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
JACOCO = 'org.jacoco:jacoco-maven-plugin:0.8.13'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--maven-home', type=Path, required=True)
    parser.add_argument('--repository', type=Path, required=True)
    parser.add_argument('--acquire', action='store_true', help='Explicit native online fixture setup before Oyzu tests')
    args = parser.parse_args()
    cli = args.cli.resolve()
    env = dict(os.environ, MAVEN_HOME=str(args.maven_home.resolve()),
               MAVEN_ARGS='-Dmaven.repo.local='+str(args.repository.resolve()))
    launcher = args.maven_home.resolve()/('bin/mvn.cmd' if os.name == 'nt' else 'bin/mvn')
    spec = importlib.util.spec_from_file_location('build_scenarios_runner', ROOT/'tooling/test-build-scenarios.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    with tempfile.TemporaryDirectory(prefix='oyzu Maven direct ') as directory:
        project = Path(directory)/'project'
        shutil.copytree(ROOT/'examples/builds/java-maven-reactor/project', project)
        app_pom = project/'app/pom.xml'
        shutil.copyfile(ROOT/'examples/builds/java-maven-reactor/variants/reporting/app.pom.xml', app_pom)
        originals = {p: p.read_bytes() for p in project.rglob('*') if p.is_file()}
        if args.acquire:
            subprocess.run([str(launcher), '-B', '-ntp', JACOCO+':prepare-agent', 'test', JACOCO+':report'],
                           cwd=project, env=env, check=True, timeout=600)
        # Provisioning is over; all product Maven invocations use offline mode.
        def invoke(*argv, success=True):
            result = subprocess.run([str(cli), '-C', str(project), *argv], env=env,
                                    capture_output=True, text=True, encoding='utf-8', timeout=300)
            assert (result.returncode == 0) == success, (argv, result.stdout, result.stderr)
            return result

        def bundle(success=True):
            manifest = harness.validate(project/'dist')
            assert manifest['status'] == ('succeeded' if success else 'failed')
            assert not manifest['artifacts'] and len(manifest['reports']) == 4
            tests = [r for r in manifest['reports'] if r['kind'] == 'test']
            assert sum(r['summary']['failed'] for r in tests) == (0 if success else 1)
            assert sum(r['summary']['passed'] for r in tests) == (2 if success else 1)
            assert all(r['summary']['covered'] > 0 for r in manifest['reports'] if r['kind'] == 'coverage')
            invoke('inspect', 'dist')
            return manifest

        listed = json.loads(invoke('run', 'list', '--json').stdout)
        assert listed['project:test']['argv'] == ['mvn', '-B', 'test']
        invoke('run', 'test')
        bundle()
        assert all(p.read_bytes() == data for p, data in originals.items())
        (project/'oyzu.toml').write_text('[tasks."project:post_test"]\nargv=["python","-c","from pathlib import Path; Path(\'hook-ran\').write_text(\'yes\')"]\n', encoding='utf-8')
        invoke('run', 'project:test')
        assert bundle()['actions'][-1]['id'] == 'project:post_test'
        assert (project/'hook-ran').is_file()
        (project/'hook-ran').unlink()
        source = project/'app/src/test/java/example/AppTest.java'
        source.write_text(source.read_text(encoding='utf-8').replace('Hello, Oyzu!', 'wrong result'), encoding='utf-8')
        (project/'app/target/test-classes/example/AppTest.class').unlink()
        invoke('run', 'test', success=False)
        assert bundle(False)['actions'][-1]['status'] == 'blocked'
        assert not (project/'hook-ran').exists()
        print('Maven direct reactor: native custom report directories, per-module JUnit/JaCoCo, reruns, hooks, failure evidence, unchanged POMs and inspected bundles passed')


if __name__ == '__main__':
    main()
