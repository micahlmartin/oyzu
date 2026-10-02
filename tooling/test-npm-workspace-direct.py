"""Authored npm workspace -> native tests -> per-package evidence -> inspection."""
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
    with tempfile.TemporaryDirectory(prefix='oyzu workspace direct ') as directory:
        project = Path(directory)/'project'
        shutil.copytree(ROOT/'examples/builds/node-workspace/project', project)
        npm = shutil.which('npm.cmd' if os.name == 'nt' else 'npm')
        subprocess.run([npm, 'ci', '--ignore-scripts', '--no-audit', '--no-fund'], cwd=project, check=True)
        originals = {p: p.read_bytes() for p in project.rglob('package*.json') if 'node_modules' not in p.parts}

        def invoke(*argv, success=True):
            result = subprocess.run([str(cli), '-C', str(project), *argv], capture_output=True,
                                    text=True, encoding='utf-8', timeout=120)
            assert (result.returncode == 0) == success, (argv, result.stdout, result.stderr)
            return result

        listed = json.loads(invoke('run', 'list', '--json').stdout)
        assert listed['project:test']['argv'] == ['npm', 'run', 'test', '--workspaces']
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
        print('EX-020 direct tests: native member scripts, distinct JUnit/coverage, hooks, failed/sibling reports, unchanged metadata and inspected bundles passed')


if __name__ == '__main__':
    main()
