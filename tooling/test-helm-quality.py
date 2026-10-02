"""Real yamlfmt through the compiled CLI; no Helm cluster or container is required."""
import argparse
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
    parser.add_argument('--yamlfmt', type=Path, required=True)
    args = parser.parse_args()
    cli = str(args.cli.resolve())
    env = dict(os.environ, PATH=str(args.yamlfmt.resolve().parent)+os.pathsep+os.environ['PATH'])
    with tempfile.TemporaryDirectory(prefix='oyzu helm quality ') as temporary:
        base = Path(temporary)
        project = base/'project'
        shutil.copytree(ROOT/'examples/builds/helm-chart/project', project)
        (project/'chart/tests').mkdir()
        shutil.copyfile(ROOT/'examples/builds/helm-chart/variants/deployment_test.yaml', project/'chart/tests/deployment_test.yaml')
        (project/'chart/tests/__snapshot__').mkdir()
        (project/'chart/tests/__snapshot__/untouched.yaml').write_text('invalid yaml: [\n')
        def invoke(*argv, success=True):
            result = subprocess.run([cli, '--root', str(project), *argv], env=env,
                                    capture_output=True, text=True, encoding='utf-8', timeout=120)
            assert (result.returncode == 0) == success, result.stdout+result.stderr
            return result.stdout+result.stderr
        def tree():
            return {p.relative_to(project).as_posix():p.read_bytes() for p in project.rglob('*') if p.is_file()}
        tasks = json.loads(invoke('run', 'list', '--json'))
        assert tasks['project:format-check']['build_stage'] and not tasks['project:format-check']['mutates_source']
        assert tasks['project:format']['mutates_source'] and not tasks['project:format']['build_stage']
        before = tree()
        invoke('run', 'format-check')
        assert tree() == before
        values = project/'chart/values.yaml'
        values.write_text('replicaCount:    1\nimage: {repository: example, tag: latest}\n')
        bad = tree()
        assert 'Formatting differs' in invoke('run', 'format-check', success=False)
        assert tree() == bad
        invoke('run', 'format')
        invoke('run', 'format-check')
        assert values.read_bytes() != bad['chart/values.yaml']
        assert (project/'chart/templates/deployment.yaml').read_bytes() == before['chart/templates/deployment.yaml']
        assert (project/'chart/tests/__snapshot__/untouched.yaml').read_bytes() == before['chart/tests/__snapshot__/untouched.yaml']
        # Invalid YAML remains a native error; format checks never repair it.
        values.write_text('replicaCount: [invalid\n')
        bad = tree()
        invoke('run', 'format-check', success=False)
        assert tree() == bad
        values.write_bytes(before['chart/values.yaml'])
        # Ambient formatter configuration must not affect captured/development parity.
        (base/'.yamlfmt').write_text('formatter:\n  indent: 8\n')
        invoke('run', 'format-check')
        (project/'.yamlfmt').write_text('formatter:\n  indent: 8\n')
        assert 'explicit format/format-check task override' in invoke('run', 'format-check', success=False)
        # The existing task override remains the escape hatch, with no new DSL.
        (project/'oyzu.toml').write_text('[tasks."project:format-check"]\nargv=["python","-c","print(123)"]\n')
        assert '123' in invoke('run', 'format-check')
        for variant in ['subchart-only', 'packaged-only']:
            project = base/variant
            shutil.copytree(ROOT/f'examples/builds/helm-chart/variants/{variant}', project)
            before = tree()
            invoke('run', 'format-check')
            assert tree() == before
        print('Helm YAML quality: native checks, explicit formatting, template preservation, invalid YAML, config isolation and overrides passed')


if __name__ == '__main__':
    main()
