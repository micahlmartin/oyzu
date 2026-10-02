"""Real Ant assertion/JUnit projects -> CLI -> JaCoCo/JUnit bundle -> inspection."""
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
    parser.add_argument('--tools', type=Path, help='Directory from provision-ant-reporting.py; otherwise use ANT_HOME/JACOCO_HOME')
    args = parser.parse_args()
    if args.tools:
        os.environ['ANT_HOME'] = str(args.tools.resolve()/'apache-ant-1.10.18')
        os.environ['JACOCO_HOME'] = str(args.tools.resolve()/'jacoco')
    cli = args.cli.resolve()
    spec = importlib.util.spec_from_file_location('build_scenarios_runner', ROOT/'tooling/test-build-scenarios.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    fixtures = [('conventional', ROOT/'examples/builds/java-ant/conventional'),
                ('custom', ROOT/'examples/builds/java-ant/custom'),
                ('junit', ROOT/'tooling/fixtures/ant-junit')]
    with tempfile.TemporaryDirectory(prefix='oyzu Ant direct ') as directory:
        for name, fixture in fixtures:
            project = Path(directory)/name
            shutil.copytree(fixture, project)
            original = {p: p.read_bytes() for p in project.rglob('*') if p.is_file()}

            def invoke(*argv, success=True):
                result = subprocess.run([str(cli), '-C', str(project), *argv], capture_output=True,
                                        text=True, encoding='utf-8', timeout=120)
                assert (result.returncode == 0) == success, (name, argv, result.stdout, result.stderr)
                return result

            def bundle(success=True):
                manifest = harness.validate(project/'dist')
                assert manifest['status'] == ('succeeded' if success else 'failed')
                assert not manifest['artifacts']
                assert manifest['extensions']['oyzu.dev/invocation']['isolation'] == 'none'
                reports = {r['kind']: r for r in manifest['reports']}
                assert reports['test']['summary']['failed'] == (0 if success else 1)
                assert reports['test']['summary']['passed'] == ((2 if success else 1) if name == 'junit' else (1 if success else 0))
                assert reports['coverage']['format'] == 'jacoco'
                assert reports['coverage']['summary']['covered'] > 0
                invoke('inspect', 'dist')
                return manifest

            tasks = json.loads(invoke('run', 'list', '--json').stdout)
            task_id = 'test' if 'test' in tasks else 'project:test'
            hook_id = 'post_test' if task_id == 'test' else 'project:post_test'
            assert tasks[task_id]['availability'] is None
            invoke('run', 'test')
            bundle()
            assert all(p.read_bytes() == data for p, data in original.items())

            # Shared task hooks still surround the real native target.
            config = project/'oyzu.toml'
            with config.open('a', encoding='utf-8', newline='\n') as stream:
                stream.write(f'\n[tasks."{hook_id}"]\nargv=["python","-c","from pathlib import Path; Path(\'hook-ran\').write_text(\'yes\')"]\n')
            invoke('run', task_id)
            assert (project/'hook-ran').is_file()
            assert bundle()['actions'][-1]['id'] == hook_id
            (project/'hook-ran').unlink()
            source = project/('test/example/CalculatorTest.java' if name == 'junit' else 'test/example/GreetingCheck.java')
            text = source.read_text(encoding='utf-8')
            source.write_text(text.replace('assertEquals(1,', 'assertEquals(99,') if name == 'junit'
                              else text.replace('Hello, Oyzu!', 'wrong greeting'), encoding='utf-8')
            # Remove this fixture's compiled test only, so native Ant recompiles
            # even on filesystems whose modification time granularity is coarse.
            (project/'build/classes/example'/('CalculatorTest.class' if name == 'junit' else 'GreetingCheck.class')).unlink()
            invoke('run', 'test', success=False)
            assert bundle(False)['actions'][-1]['status'] == 'blocked'
            assert not (project/'hook-ran').exists()
            print(f'Ant {name}: native tests, JUnit/JaCoCo, hooks, real failure, unchanged sources and inspected bundles passed', flush=True)


if __name__ == '__main__':
    main()
