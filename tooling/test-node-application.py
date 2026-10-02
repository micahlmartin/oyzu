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
        # Configured output is obtained from the real Vite API, not from an
        # emulated config parser. The package manager retains lifecycle hooks.
        shutil.rmtree(project/'public-site')
        config = project/'vite.config.mjs'
        shutil.copyfile(ROOT/'examples/builds/materialize-directory/variants/vite-config/vite.config.mjs', config)
        package['scripts']['build'] = 'vite build'
        for hook in ['prebuild', 'postbuild']:
            package['scripts'][hook] = f'''node -e "require('node:fs').appendFileSync('.oyzu-build/{hook}.txt','once\\n')"'''
        package_file.write_text(json.dumps(package))
        runtime_dir = base/'runtime'
        runtime_dir.mkdir()
        shutil.copyfile(ROOT/'src/builders/node/runtime/vite.mjs', runtime_dir/'node-vite.mjs')
        shutil.copyfile(runtime, runtime_dir/'node-application.mjs')
        adapter = str(runtime_dir/'node-vite.mjs')
        record = base/'vite-output.json'
        env['OYZU_NODE_VITE_STATE'] = str(base/'vite-state.json')
        env['OYZU_NODE_VITE_RECORD'] = str(record)
        env['OYZU_VERSION'] = '1.0.0-dev.g1234'
        run(['node', adapter, 'prepare', str(record), 'vite build', ''])
        run([shutil.which('npm.cmd' if os.name=='nt' else 'npm'), 'run', 'build'])
        metadata = json.loads(record.read_text())
        assert metadata['output']=='web/production' and metadata['mode']=='production'
        assert metadata['version']==env['OYZU_VERSION'] and metadata['toolVersion'].startswith('8.')
        for hook in ['prebuild', 'postbuild']:
            assert (project/f'.oyzu-build/{hook}.txt').read_text()=='once\n'
        assert (project/'web/production/native-mode.txt').read_text()=='production'
        # Implicit checks consume the recorded native location after build.
        (project/'web/production/not-source.js').write_text('invalid JavaScript @@@')
        # Use the compiled task adapter with its detected native build script.
        package_file.write_text(json.dumps(package))
        run([cli, 'run', 'lint'])
        run([cli, 'run', 'format-check'])
        run(['node', adapter, 'package', str(record), str(base/'configured')])
        assert (base/'configured/native-mode.txt').read_text()=='production'
        metadata['version']='wrong-version'
        record.write_text(json.dumps(metadata))
        run(['node', adapter, 'package', str(record), str(base/'bad-record')], False)
        record.unlink()
        # Before a build, explicit quality checks resolve native config; static
        # discovery still observes files without evaluating them.
        run([cli, 'run', 'lint'])
        run([cli, 'run', 'format-check'])
        config.write_text('throw Error("native-config-failure");\n')
        run([cli, 'run', 'list', '--json'])
        failed = run(['node', adapter, 'build'], False)
        assert 'native-config-failure' in failed.stderr and not record.exists()
        config.write_text('export default {build:{outDir:"../escaped"}};\n')
        run(['node', adapter, 'build'], False)
        assert not (base/'escaped').exists() and not record.exists()
        for settings in ['{write:false}', '{watch:{}}', '{rolldownOptions:{output:{dir:"../escaped"}}}']:
            config.write_text(f'export default {{build:{settings}}};\n')
            run(['node', adapter, 'build'], False)
            assert not record.exists() and not (base/'escaped').exists()
        # Native CLI output override retains precedence over config.
        config.write_text('export default {build:{outDir:"native-default"}};\n')
        state = Path(env['OYZU_NODE_VITE_STATE'])
        data = json.loads(state.read_text())
        data['outDir'] = 'native-override'
        state.write_text(json.dumps(data))
        run(['node', adapter, 'build'])
        assert json.loads(record.read_text())['output']=='native-override'
        assert (project/'native-override/index.html').is_file()
    print('Native Vite CLI tasks, configuration/API output evidence, lifecycle hooks, plugin asset, quality exclusions, output override, metadata/failure rejection and staging passed; isolated build acceptance remains separate.')


if __name__=='__main__':
    main()
