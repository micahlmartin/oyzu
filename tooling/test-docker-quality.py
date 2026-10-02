"""Real native Dockerfile quality tasks through the compiled CLI; no daemon."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--tools', type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    env = {**os.environ, 'PATH': str(args.tools.resolve()) + os.pathsep + os.environ.get('PATH', '')}
    with tempfile.TemporaryDirectory(prefix='oyzu Docker quality ') as temporary:
        project = Path(temporary)
        source = project/'Dockerfile'
        good = 'FROM scratch\nCOPY greeting.txt /greeting.txt\n'
        source.write_text(good, newline='\n')
        (project/'greeting.txt').write_text('Hello\n')

        def run(*argv, success=True, environment=env):
            result = subprocess.run([str(cli), '-C', str(project), *argv], env=environment,
                                    capture_output=True, text=True, encoding='utf-8', timeout=60)
            assert (result.returncode == 0) == success, (argv, result.stdout, result.stderr)
            return result

        listed = json.loads(run('run', 'list', '--json', environment={**env, 'PATH':''}).stdout)
        for name in ['lint', 'format-check', 'format']:
            assert listed['project:'+name]['availability'] is None
        assert listed['project:lint']['build_stage'] and listed['project:format-check']['build_stage']
        assert listed['project:format']['mutates_source'] and not listed['project:format']['build_stage']
        before = source.read_bytes()
        run('run', 'lint')
        run('run', 'format-check')
        assert source.read_bytes() == before
        # Authored standalone fixtures must pass without inheriting this repo's
        # EditorConfig; captured builds transport only their selected inputs.
        variants = Path(__file__).resolve().parents[1] / 'examples/builds/docker-offline/variants'
        for name in ['provisioned-base', 'argument-base', 'image-aliases']:
            source.write_bytes((variants / name / 'Dockerfile').read_bytes())
            original = source.read_bytes()
            run('run', 'lint')
            run('run', 'format-check')
            assert source.read_bytes() == original
        source.write_text(good, newline='\n')
        run('run', 'lint', success=False, environment={**env, 'PATH':''})

        source.write_text('FROM scratch\nWORKDIR relative\n', newline='\n')
        before = source.read_bytes()
        failure = run('run', 'lint', success=False)
        assert 'DL3000' in failure.stdout + failure.stderr
        assert source.read_bytes() == before
        (project/'.hadolint.yaml').write_text('ignored: [DL3000]\n')
        run('run', 'lint')
        (project/'.hadolint.yaml').write_text('ignored: [\n')
        run('run', 'lint', success=False)
        (project/'.hadolint.yaml').unlink()

        source.write_text('from scratch\nCOPY    greeting.txt     /greeting.txt\n', newline='\n')
        before = source.read_bytes()
        run('run', 'format-check', success=False)
        assert source.read_bytes() == before
        run('run', 'format-check', '--write', success=False)
        assert source.read_bytes() == before
        run('run', 'format')
        run('run', 'format-check')
        assert source.read_text() == good
        # Native EditorConfig can explicitly choose a different EOF convention.
        (project/'.editorconfig').write_text('root = true\n[Dockerfile]\ninsert_final_newline = false\n')
        run('run', 'format-check', success=False)
        run('run', 'format')
        assert source.read_text() == good.rstrip('\n')
        run('run', 'format-check')
        # User task replacement retains the shared task/hook execution path.
        (project/'oyzu.toml').write_text('[tasks.lint]\nargv=["dockerfmt", "--check", "Dockerfile"]\n')
        run('run', 'lint')
        print('Native Docker quality passed: static tasks, real lint/format failures, unchanged check inputs, explicit writes, native configuration, missing tools and task replacement')


if __name__ == '__main__':
    main()
