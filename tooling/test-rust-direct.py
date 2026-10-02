"""Real Cargo/nextest/LLVM coverage -> compiled CLI -> inspected test bundles."""
import argparse
import importlib.util
import json
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
    with tempfile.TemporaryDirectory(prefix='oyzu Rust direct ') as directory:
        for example in ['rust-app', 'rust-workspace']:
            project = Path(directory)/example
            shutil.copytree(ROOT/'examples/builds'/example/'project', project)
            config = project/'.config/nextest.toml'
            config.parent.mkdir()
            config.write_text('[profile.default]\nfail-fast=false\ntest-threads=1\n', encoding='utf-8')
            original = {p: p.read_bytes() for p in project.rglob('*') if p.is_file()}

            def invoke(*argv, success=True):
                result = subprocess.run([str(cli), '-C', str(project), *argv], capture_output=True,
                                        text=True, encoding='utf-8', timeout=240)
                assert (result.returncode == 0) == success, (example, argv, result.stdout, result.stderr)
                return result

            tasks = json.loads(invoke('run', 'list', '--json').stdout)
            assert tasks['project:test']['argv'] == ['cargo', 'test', '--locked', '--workspace']
            invoke('run', 'test')
            manifest = harness.validate(project/'dist')
            assert manifest['status'] == 'succeeded' and not manifest['artifacts']
            assert manifest['extensions']['oyzu.dev/invocation']['isolation'] == 'none'
            assert next(r for r in manifest['reports'] if r['kind'] == 'coverage')['summary']['covered'] > 0
            assert all(r['summary']['failed'] == 0 for r in manifest['reports'] if r['kind'] == 'test')
            assert all(p.read_bytes() == data for p, data in original.items())
            invoke('inspect', 'dist')

            if example == 'rust-workspace':
                doctest, = [r for r in manifest['reports'] if r['id'].endswith(':doctest')]
                assert doctest['summary']['passed'] == 2
                source = project/'core/src/lib.rs'
                good = source.read_text(encoding='utf-8')
                source.write_text(good.replace('!example_core::greeting().is_empty()', 'example_core::greeting().is_empty()'), encoding='utf-8')
                invoke('run', 'test', success=False)
                failed = harness.validate(project/'dist')
                assert failed['status'] == 'failed'
                assert next(r for r in failed['reports'] if r['id'].endswith(':doctest'))['summary']['failed'] == 1
                assert next(r for r in failed['reports'] if r['id'] == 'project:test')['summary']['failed'] == 0
            else:
                source = project/'src/main.rs'
                source.write_text(source.read_text(encoding='utf-8')+'\n#[test] fn actual_failure() { assert_eq!(1, 2); }\n', encoding='utf-8')
                invoke('run', 'test', success=False)
                failed = harness.validate(project/'dist')
                assert next(r for r in failed['reports'] if r['kind'] == 'test')['summary']['failed'] == 1
                assert next(r for r in failed['reports'] if r['kind'] == 'coverage')['summary']['covered'] > 0
            invoke('inspect', 'dist')
            print(f'{example}: native tests/coverage, current failure evidence, source/config preservation and bundle inspection passed', flush=True)


if __name__ == '__main__':
    main()
