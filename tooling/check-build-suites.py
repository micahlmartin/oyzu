"""Check acceptance registration and CI coverage without executing product builds."""
import ast
from functools import partial
from pathlib import Path
import runpy

import yaml

ROOT = Path(__file__).resolve().parents[1]


def main():
    runner = runpy.run_path(str(ROOT / 'tooling/test-build-scenarios.py'))
    suites, cases, select = (runner[name] for name in ('SUITES', 'CASES', 'select_cases'))
    assert len(set(suites)) == len(suites) and 'all' not in suites
    assert len({name for _, name, _ in cases}) == len(cases), 'duplicate check identity'
    assert all(group in suites for group, _, _ in cases), 'unknown suite registration'
    assert all(select(suite) for suite in suites), 'empty suite'
    assert len(select('all')) == sum(len(select(suite)) for suite in suites)
    callbacks = [check.func if isinstance(check, partial) else check for _, _, check in cases]
    assert len(set(callbacks)) == len(callbacks), 'a check is registered more than once'
    registered = {(check.__module__.split('.')[-1], check.__name__) for check in callbacks}
    # Modules exposing the shared acceptance entry point must be in the catalog.
    # Local helpers with different signatures remain owned by their caller.
    signature = ['root', 'base', 'invoke', 'validate', 'source_files', 'verified']
    for path in (ROOT / 'tooling/build_scenarios').glob('*.py'):
        tree = ast.parse(path.read_text())
        for node in tree.body:
            if isinstance(node, ast.FunctionDef) and node.name == 'verify':
                if [arg.arg for arg in node.args.args] == signature:
                    assert (path.stem, node.name) in registered, f'unregistered acceptance entry point: {path.name}'
    workflow = yaml.safe_load((ROOT / '.github/workflows/build.yml').read_text())
    job = workflow['jobs']['build-scenarios']
    matrix = job['strategy']['matrix']['suite']
    assert len(matrix) == len(suites) and set(matrix) == set(suites), 'CI omits or duplicates a suite'
    assert job['needs'] == 'build-cli', 'acceptance must use the compiled CLI'
    assert job['strategy']['fail-fast'] is False, 'one failure must not cancel unrelated suites'
    gate = workflow['jobs']['build-scenarios-complete']
    assert gate['needs'] == 'build-scenarios' and gate['if'] == 'always()'
    print(f'Acceptance inventory passed: {len(suites)} CI suites, {len(cases)} registered check groups. No native builds executed.')


if __name__ == '__main__':
    main()
