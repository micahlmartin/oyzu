"""Real Helm host tests through the CLI, with inspected test-only dist bundles."""
import argparse
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--helm', default='helm')
    args = parser.parse_args()
    cli = args.cli.resolve()
    helm = Path(shutil.which(args.helm) or args.helm).resolve()
    env = {**os.environ, 'PATH': str(helm.parent) + os.pathsep + os.environ.get('PATH', '')}
    validate = runpy.run_path(str(ROOT / 'tooling/test-build-scenarios.py'))['validate']

    with tempfile.TemporaryDirectory(prefix='oyzu helm direct ') as temporary:
        base = Path(temporary)
        project = base / 'application'
        shutil.copytree(ROOT / 'examples/builds/helm-chart/project', project)
        # Explicit native provisioning is a development prerequisite, not a
        # hidden download/installation performed by the test command.
        subprocess.run([str(helm), 'dependency', 'build', str(project / 'chart'), '--skip-refresh'],
                       env=env, check=True, capture_output=True, text=True)

        def invoke(*argv, success=True):
            result = subprocess.run([str(cli), '-C', str(project), '--json', *argv],
                                    env=env, capture_output=True, text=True, timeout=120)
            assert (result.returncode == 0) == success, (argv, result.stdout, result.stderr)
            return json.loads(result.stdout) if result.stdout.strip() else None

        def bundle(status='succeeded', tests=1, target='project'):
            manifest = validate(project / 'dist')
            assert manifest['status'] == status and manifest['artifacts'] == []
            assert len(manifest['reports']) == tests
            assert all(r['kind'] == 'test' and r['status'] == 'collected' for r in manifest['reports'])
            record, = [t for t in manifest['targets'] if t['id'] == target]
            assert record['extensions']['oyzu.dev/coverage-applicability']['status'] == 'inapplicable'
            assert manifest['extensions']['oyzu.dev/invocation']['isolation'] == 'none'
            plan = json.loads((project / 'dist/plan.json').read_text(encoding='utf-8'))
            assert plan['policy']['requiredChecks'] == ['tests']
            invoke('inspect', 'dist')
            return manifest

        invoke('run', 'test')
        manifest = bundle()
        assert manifest['reports'][0]['summary']['passed'] == 1
        # A real values validation failure must retain the native failure report.
        values = project / 'chart/values.yaml'
        original = values.read_bytes()
        values.write_bytes(original.replace(b'replicaCount: 1', b'replicaCount: 0'))
        invoke('run', 'test', success=False)
        assert bundle('failed')['reports'][0]['summary']['failed'] == 1
        values.write_bytes(original)

        # Named target and hooks use the same report transaction. The post-hook
        # must not erase the target's coverage-applicability record.
        (project / 'build.yaml').write_text('chart:\n  uses: helm/chart\n  path: chart\n', encoding='utf-8')
        (project / 'oyzu.toml').write_text(
            '[tasks."chart:post_test"]\nargv=' + json.dumps([sys.executable, '-c', "print('after chart test')"]) + '\n',
            encoding='utf-8',
        )
        (project / 'chart/tests').mkdir()
        suite = project / 'chart/tests/deployment_test.yaml'
        shutil.copyfile(ROOT / 'examples/builds/helm-chart/variants/deployment_test.yaml', suite)
        invoke('run', 'chart:test')
        manifest = bundle(tests=2, target='chart')
        assert all(r['summary']['passed'] == 1 for r in manifest['reports'])
        assert [a['id'] for a in manifest['actions']] == ['chart:test', 'chart:post_test']
        suite.write_text(suite.read_text(encoding='utf-8').replace('value: 1', 'value: 99'), encoding='utf-8')
        invoke('run', 'chart:test', success=False)
        manifest = bundle('failed', tests=2, target='chart')
        assert sum(r['summary']['failed'] for r in manifest['reports']) == 1
        assert manifest['actions'][-1]['status'] == 'blocked'

        project = base / 'library'
        shutil.copytree(ROOT / 'examples/builds/helm-chart/project/labels', project)
        invoke('run', 'test')
        assert bundle()['reports'][0]['summary']['passed'] == 1
        (project / 'oyzu.toml').write_text('[checks]\ncoverageMinimum=1\n', encoding='utf-8')
        invoke('run', 'test', success=False)
    print('Helm direct tests: application/library validation, native unittest, failed JUnit, qualified hooks, coverage inapplicability and inspected bundles passed')


if __name__ == '__main__':
    main()
