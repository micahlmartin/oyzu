"""Native Vite development tasks and output staging; no isolated-build claim."""
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
    runtime = str(ROOT/'src/builders/node/runtime/application.mjs')
    env = {**os.environ, 'OYZU_NODE_QUALITY_HOME':str(ROOT/'tooling/images/node-quality')}
    with tempfile.TemporaryDirectory(prefix='oyzu vite ') as temporary:
        base = Path(temporary)
        project = base/'frontend'
        shutil.copytree(ROOT/'examples/builds/materialize-directory/variants/vite/frontend', project,
                        ignore=shutil.ignore_patterns('node_modules', 'dist'))

        def run(command, success=True):
            result = subprocess.run(command, cwd=project, env=env, capture_output=True, text=True, timeout=180)
            assert (result.returncode==0)==success, f'{command}\n{result.stdout}\n{result.stderr}'
            return result

        # Explicit native dependency provisioning for the probe, not CLI tool installation.
        run([shutil.which('npm.cmd' if os.name=='nt' else 'npm'), 'ci', '--ignore-scripts', '--no-audit', '--no-fund'])
        source = {p.relative_to(project):p.read_bytes() for p in project.rglob('*')
                  if p.is_file() and 'node_modules' not in p.relative_to(project).parts}
        tasks = json.loads(run([cli, 'run', 'list', '--json']).stdout)
        assert tasks['project:build']['argv']==['npm', 'run', 'build']
        for name in ['build', 'test', 'lint', 'format-check']:
            run([cli, 'run', name])
        assert (project/'dist/index.html').is_file()
        assert list((project/'dist/assets').glob('*.js'))
        for relative, data in source.items():
            assert (project/relative).read_bytes()==data
        stage = base/'artifact'
        run(['node', runtime, 'dist', str(stage)])
        assert (stage/'index.html').read_bytes()==(project/'dist/index.html').read_bytes()
        assert not (stage/'dist').exists()
        run(['node', runtime, 'dist', str(stage)], False)
        run(['node', runtime, '../outside', str(base/'escaped')], False)
        run(['node', runtime, 'missing', str(base/'missing')], False)
        package_file = project/'package.json'
        package = json.loads(package_file.read_text())
        package['scripts']['build']='vite build --outDir public-site'
        package_file.write_text(json.dumps(package))
        run([cli, 'run', 'build'])
        (project/'public-site/not-source.js').write_text('invalid JavaScript @@@')
        run([cli, 'run', 'lint'])
        run([cli, 'run', 'format-check'])
        run(['node', runtime, 'public-site', str(base/'custom')])
        assert (base/'custom/index.html').is_file()
        # Host checks exercise links only where the OS permits creating them.
        if os.name!='nt':
            (project/'dist/linked').symlink_to(base/'artifact/index.html')
            run(['node', runtime, 'dist', str(base/'linked')], False)
            (project/'dist/linked').unlink()
            os.mkfifo(project/'dist/pipe')
            run(['node', runtime, 'dist', str(base/'special')], False)
    print('Native Vite CLI tasks, browser lint defaults, read-only checks, native outDir and output staging passed; isolated build acceptance remains separate.')


if __name__=='__main__':
    main()
