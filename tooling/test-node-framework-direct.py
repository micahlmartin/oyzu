"""Real framework -> compiled CLI -> JUnit/coverage bundle -> inspection."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    spec = importlib.util.spec_from_file_location('build_scenarios_runner', ROOT/'tooling/test-build-scenarios.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    env = {**os.environ, 'OYZU_NODE_REPORTING_HOME': str(ROOT/'tooling/images/node-quality')}
    with tempfile.TemporaryDirectory(prefix='oyzu framework direct ') as directory:
        for framework, script in [('jest', 'jest --ci'), ('vitest', 'vitest run'), ('mocha', 'mocha')]:
            fixture = ROOT/'tooling/fixtures'/framework
            assert (fixture/'node_modules').is_dir(), f'Provision dependencies in {fixture} first'
            project = Path(directory)/framework
            # npm's Unix .bin entries are links into their package. Dereferencing
            # them relocates ESM launchers and breaks their relative imports.
            shutil.copytree(fixture, project, symlinks=True)
            package_path = project/'package.json'
            package = json.loads(package_path.read_text(encoding='utf-8'))
            package['scripts'].pop('test', None)
            package_path.write_text(json.dumps(package), encoding='utf-8')

            def invoke(*argv, success=True):
                result = subprocess.run([str(cli), '-C', str(project), *argv], env=env,
                                        capture_output=True, text=True, encoding='utf-8', timeout=120)
                assert (result.returncode == 0) == success, (framework, argv, result.stdout, result.stderr)
                return result

            def bundle(success=True):
                manifest = harness.validate(project/'dist')
                assert manifest['status'] == ('succeeded' if success else 'failed')
                assert not manifest['artifacts']
                reports = {r['kind']: r for r in manifest['reports']}
                assert reports['test']['summary']['passed'] == (1 if success else 0)
                assert reports['test']['summary']['failed'] == (0 if success else 1)
                assert reports['coverage']['summary']['covered'] > 0
                assert 'greeting.js' in (project/'dist'/reports['coverage']['path']).read_text(encoding='utf-8')
                assert manifest['extensions']['oyzu.dev/invocation']['isolation'] == 'none'
                invoke('inspect', 'dist')
                return manifest

            listed = json.loads(invoke('run', 'list', '--json').stdout)
            assert listed['project:test']['availability'] is None
            invoke('run', 'test')
            bundle()

            # Native scripts, their pre/post lifecycle and Oyzu hooks all remain
            # real commands; collection happens after the Oyzu post hook.
            (project/'lifecycle.cjs').write_text(
                "require('node:fs').appendFileSync('events.txt',process.argv[2]+'\\n');", encoding='utf-8')
            package['scripts'].update({'test': script, 'pretest': 'node lifecycle.cjs pre', 'posttest': 'node lifecycle.cjs post'})
            package_path.write_text(json.dumps(package), encoding='utf-8')
            (project/'oyzu.toml').write_text('[tasks."project:post_test"]\nargv=["node","lifecycle.cjs","oyzu-post"]\n', encoding='utf-8')
            invoke('run', 'project:test')
            assert [a['id'] for a in bundle()['actions']] == ['project:test', 'project:post_test']
            assert (project/'events.txt').read_text(encoding='utf-8').splitlines() == ['pre', 'post', 'oyzu-post']

            # A failed native assertion must retain current JUnit and coverage,
            # fail the command, and skip both native and Oyzu success-only hooks.
            test = project/'test/greeting.test.js'
            test.write_text(test.read_text(encoding='utf-8').replace('Hello, Oyzu!', 'wrong greeting'), encoding='utf-8')
            invoke('run', 'test', success=False)
            failed = bundle(False)
            assert failed['actions'][-1]['status'] == 'blocked'
            assert (project/'events.txt').read_text(encoding='utf-8').splitlines() == ['pre', 'post', 'oyzu-post', 'pre']
            print(f'{framework}: implicit and native npm tests, hooks, JUnit/coverage, failure bundle and inspection passed', flush=True)


if __name__ == '__main__':
    main()
