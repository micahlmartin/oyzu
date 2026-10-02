"""EX-020 workspace -> native manager tests -> package evidence -> inspection."""
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
    parser.add_argument('--manager', choices=['npm', 'pnpm', 'yarn'], default='npm')
    args = parser.parse_args()
    cli = args.cli.resolve()
    spec = importlib.util.spec_from_file_location('build_scenarios_runner', ROOT/'tooling/test-build-scenarios.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    with tempfile.TemporaryDirectory(prefix='oyzu workspace direct ') as directory:
        project = Path(directory)/'project'
        shutil.copytree(ROOT/'examples/builds/node-workspace/project', project)
        env = dict(os.environ)
        if args.manager == 'npm':
            native = [shutil.which('npm.cmd' if os.name == 'nt' else 'npm')]
            install = ['ci', '--ignore-scripts', '--no-audit', '--no-fund']
            expected = ['npm', 'run', 'test', '--workspaces']
        else:
            home = ROOT/f'tooling/images/node-{args.manager}/node_modules'
            env['PATH'] = str(home/'.bin')+os.pathsep+env['PATH']
            (project/'package-lock.json').unlink()
            root_package = json.loads((project/'package.json').read_text(encoding='utf-8'))
            if args.manager == 'pnpm':
                native = ['node', str(home/'pnpm/bin/pnpm.cjs'), '--config.manage-package-manager-versions=false']
                root_package.pop('workspaces')
                root_package['packageManager'] = 'pnpm@10.11.0'
                (project/'pnpm-workspace.yaml').write_text('packages:\n  - packages/*\n', encoding='utf-8')
                member = project/'packages/app/package.json'
                package = json.loads(member.read_text(encoding='utf-8'))
                package['dependencies']['@oyzu-example/shared'] = 'workspace:*'
                member.write_text(json.dumps(package), encoding='utf-8')
                expected = ['pnpm', '--recursive', 'run', 'test']
            else:
                native = ['node', str(home/'yarn/bin/yarn.js'), '--offline', '--non-interactive']
                root_package['packageManager'] = 'yarn@1.22.22'
                expected = ['yarn', 'workspaces', 'run', 'test']
            (project/'package.json').write_text(json.dumps(root_package), encoding='utf-8')
            install = ['install', '--offline', '--ignore-scripts']
        # Native fixture provisioning is separate from the product invocation.
        subprocess.run(native+install, cwd=project, env=env, check=True)
        # Exercise an implicit suite beside the app's native script.
        member = project/'packages/shared/package.json'
        package = json.loads(member.read_text(encoding='utf-8'))
        package.pop('scripts')
        member.write_text(json.dumps(package), encoding='utf-8')
        originals = {p: p.read_bytes() for p in project.rglob('*')
                     if p.is_file() and 'node_modules' not in p.parts and
                     (p.name.startswith('package') or p.name in ['pnpm-workspace.yaml', 'pnpm-lock.yaml', 'yarn.lock'])}

        def invoke(*argv, success=True):
            result = subprocess.run([str(cli), '-C', str(project), *argv], capture_output=True,
                                    env=env, text=True, encoding='utf-8', timeout=120)
            assert (result.returncode == 0) == success, (argv, result.stdout, result.stderr)
            return result

        listed = json.loads(invoke('run', 'list', '--json').stdout)
        assert listed['project:test']['argv'] == expected
        (project/'hook.cjs').write_text("require('node:fs').writeFileSync('post-ran','yes');", encoding='utf-8')
        (project/'oyzu.toml').write_text('[tasks."project:post_test"]\nargv=["node","hook.cjs"]\n', encoding='utf-8')
        invoke('run', 'test')
        manifest = harness.validate(project/'dist')
        assert manifest['status'] == 'succeeded' and not manifest['artifacts']
        assert len(manifest['reports']) == 4
        assert len({r['id'] for r in manifest['reports']}) == 4
        tests = [r for r in manifest['reports'] if r['kind'] == 'test']
        assert len(tests) == 2 and all(r['summary']['passed'] == 1 for r in tests)
        assert all(r['summary']['covered'] > 0 for r in manifest['reports'] if r['kind'] == 'coverage')
        assert (project/'post-ran').read_text(encoding='utf-8') == 'yes'
        assert all(p.read_bytes() == contents for p, contents in originals.items())
        invoke('inspect', 'dist')
        (project/'post-ran').unlink()
        failed_test = project/'packages/app/failure.test.mjs'
        failed_test.write_text("import test from 'node:test'; test('real failure',()=>{throw Error('expected')});\n", encoding='utf-8')
        invoke('run', 'test', success=False)
        failed = harness.validate(project/'dist')
        assert failed['status'] == 'failed' and len(failed['reports']) == 4
        assert sum(r['summary']['failed'] for r in failed['reports'] if r['kind'] == 'test') == 1
        assert not (project/'post-ran').exists() and failed['actions'][-1]['status'] == 'blocked'
        invoke('inspect', 'dist')
        failed_test.unlink()
        root_package = json.loads((project/'package.json').read_text(encoding='utf-8'))
        root_package['scripts'] = {'test':'node --test'}
        (project/'package.json').write_text(json.dumps(root_package), encoding='utf-8')
        invoke('run', 'test')
        aggregate = harness.validate(project/'dist')
        assert aggregate['status'] == 'succeeded' and len(aggregate['reports']) == 2
        assert sum(r['summary']['passed'] for r in aggregate['reports'] if r['kind'] == 'test') == 2
        invoke('inspect', 'dist')
        print(f'EX-020 {args.manager} direct tests: scripts/implicit suites, distinct JUnit/coverage, hooks, failed/sibling reports, unchanged metadata, root aggregation and inspected bundles passed')


if __name__ == '__main__':
    main()
