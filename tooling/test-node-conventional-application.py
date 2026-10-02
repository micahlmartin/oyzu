"""Native custom Node application tasks and directory staging, without Docker."""
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
    args = parser.parse_args()
    cli = str(args.cli.resolve())
    env = dict(os.environ, OYZU_NODE_QUALITY_HOME=str(ROOT/'tooling/images/node-quality'))
    with tempfile.TemporaryDirectory(prefix='oyzu custom application ') as directory:
        base = Path(directory)
        project = base/'project'
        shutil.copytree(ROOT/'examples/builds/materialize-directory/project', project,
                        ignore=shutil.ignore_patterns('dist', 'node_modules'))

        def run(command, cwd=project, success=True):
            result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=90)
            assert (result.returncode == 0) == success, (command, result.stdout, result.stderr)
            return result

        discovered = json.loads(run([cli, 'discover', '--json']).stdout)
        target = discovered['targets']['frontend']
        assert target['builder_selection'] == 'explicit'
        assert target['discovery']['output-profile']['selected'] == 'dist-application'
        tasks = json.loads(run([cli, 'run', 'list', '--json']).stdout)
        for operation in ['build', 'test', 'lint', 'format-check']:
            assert tasks[f'frontend:{operation}']['build_stage']
        # Normalize authored fixture formatting explicitly before taking a source
        # baseline. Build/check tasks must never repair source on the user's behalf.
        run([cli, 'run', 'frontend:format'])
        frontend = project/'frontend'
        source = {p.relative_to(frontend): p.read_bytes() for p in frontend.rglob('*') if p.is_file()}
        run([cli, 'run', 'frontend:build'])
        run([cli, 'run', 'frontend:test'])
        for operation in ['lint', 'format-check']:
            run([cli, 'run', f'frontend:{operation}'])
        runtime = str(ROOT/'src/builders/node/runtime/application.mjs')
        destination = base/'application-0.1.0-dev.probe'
        run(['node', runtime, 'dist', str(destination)], cwd=frontend)
        assert (destination/'index.html').read_bytes() == (frontend/'dist/index.html').read_bytes()
        assert not (destination/'dist').exists()
        assert set(p.name for p in destination.iterdir()) == {'index.html'}
        for path, original in source.items():
            assert (frontend/path).read_bytes() == original
        run(['node', runtime, 'dist', str(destination)], cwd=frontend, success=False)
        shutil.rmtree(frontend/'dist')
        run(['node', runtime, 'dist', str(base/'missing')], cwd=frontend, success=False)
        assert not (base/'missing').exists()
        (frontend/'dist').write_text('not a directory')
        run(['node', runtime, 'dist', str(base/'file')], cwd=frontend, success=False)
        assert not (base/'file').exists()
    print('Conventional Node app: explicit builder discovery, native build/test/quality tasks, unchanged source and directory staging/missing-output rejection passed; captured Docker acceptance remains separate.')


if __name__ == '__main__':
    main()
