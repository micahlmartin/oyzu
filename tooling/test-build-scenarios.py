"""Execute real captured-source builds through the compiled CLI and Docker.

Suites partition the implemented acceptance checks, not the complete authored
scenario catalog. Every selected check executes real product/native behavior.
"""
import argparse
from functools import partial
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from jsonschema import Draft202012Validator, FormatChecker
from build_scenarios import baselines, ant, concurrency, docker, go, gradle, helm, jest, materialization, maven, mocha, node, node_application, node_managers, node_matrix, node_preflight, node_quality, node_workspaces, python_application, python_container, python_legacy, python_quality, python_testing, rust, vitest
from build_scenarios import docker_dependencies

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def source_files(root):
    return {p.relative_to(root).as_posix(): digest(p) for p in root.rglob("*")
            if p.is_file() and not set(p.relative_to(root).parts) & {".oyzu", "dist"}}


def validate(bundle):
    records = {}
    for name in ["manifest", "plan", "envelope"]:
        path = bundle / f"{name}.json"
        if not path.exists():
            assert name == "plan"
            continue
        value = json.loads(path.read_text())
        schema = json.loads((ROOT / f"docs/contracts/v1alpha1/{name}.schema.json").read_text())
        Draft202012Validator(schema, format_checker=FormatChecker()).validate(value)
        records[name] = value
    for artifact in records["manifest"]["artifacts"] + records["manifest"]["reports"]:
        if "path" in artifact:
            path = bundle / artifact['path']
            if artifact.get('kind') == 'directory':
                entries = artifact['entries']
                assert [e['path'] for e in entries] == sorted(p.relative_to(path).as_posix() for p in path.rglob('*'))
                for entry in entries:
                    member = path/entry['path']
                    assert not member.is_symlink()
                    if entry['kind'] == 'file':
                        assert member.is_file() and digest(member) == entry['digest']
                        assert member.stat().st_size == entry['size']
                    else:
                        assert member.is_dir() and entry['size'] == 0
                # Entry keys are ASCII and numbers are bounded integers; this
                # encoding matches JCS for this deliberately narrow record.
                encoded = json.dumps(entries, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()
                assert 'sha256:'+hashlib.sha256(b'oyzu.tree.v1alpha1\0'+encoded).hexdigest() == artifact['digest']
                assert sum(e['size'] for e in entries) == artifact['size']
            else:
                assert digest(path) == artifact["digest"]
    return records["manifest"]
SUITES = ('core', 'node', 'python', 'go', 'rust', 'java', 'helm', 'docker')
# Registration retains the existing full-run order. A suite is only a selection
# of these checks; it cannot replace native outcomes or scenario expectations.
CASES = (
    ('node', 'node-overrides', node.verify_overrides),
    ('core', 'concurrency', concurrency.verify),
    ('node', 'node-preflight', node_preflight.verify),
    ('node', 'node-matrix', node_matrix.verify),
    ('node', 'node-managers', node_managers.verify),
    ('node', 'node-workspaces', node_workspaces.verify),
    ('node', 'node-quality', node_quality.verify),
    ('python', 'python-testing', python_testing.verify),
    ('node', 'jest', jest.verify),
    ('node', 'vitest', vitest.verify),
    ('node', 'mocha', mocha.verify),
    ('go', 'go', go.verify),
    ('core', 'materialization', materialization.verify),
    ('docker', 'docker', docker.verify),
    ('docker', 'docker-dependencies', docker_dependencies.verify),
    ('docker', 'node-application', node_application.verify),
    ('java', 'maven', maven.verify),
    ('java', 'gradle', gradle.verify),
    ('java', 'ant', ant.verify),
    ('helm', 'helm', helm.verify),
    ('rust', 'rust', rust.verify),
    ('python', 'python-application', python_application.verify),
    ('python', 'python-container', python_container.verify),
    ('python', 'python-legacy', python_legacy.verify),
    ('python', 'python-quality', python_quality.verify),
    ('core', 'baselines', partial(baselines.verify, digest=digest)),
)


def select_cases(suite):
    if suite not in ('all', *SUITES):
        raise ValueError(f'unknown build suite: {suite}')
    return [(name, check) for group, name, check in CASES if suite == 'all' or group == suite]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path)
    parser.add_argument("--suite", choices=('all', *SUITES), default='all')
    parser.add_argument("--list-suites", action='store_true', help='List implemented check groups without running builds')
    parser.add_argument("--evidence-dir", type=Path, help="Retain generated bundles for CI review")
    args = parser.parse_args()
    if args.list_suites:
        print(json.dumps({suite: [name for name, _ in select_cases(suite)] for suite in SUITES}, indent=2))
        return
    if args.cli is None:
        parser.error('--cli is required when executing build scenarios')
    cli = args.cli.resolve()
    verified = []
    evidence = args.evidence_dir.resolve() if args.evidence_dir else None
    if evidence:
        evidence.mkdir(parents=True, exist_ok=False)
    invocation = 0

    def invoke(root, *command, success=True, timeout=900):
        nonlocal invocation
        started = time.monotonic()
        print(f"[{root.name}] oyzu {' '.join(command)}", flush=True)
        result = subprocess.run([str(cli), "-C", str(root), *command], capture_output=True, text=True, timeout=timeout)
        print(f"[{root.name}] exit {result.returncode} after {time.monotonic()-started:.1f}s", flush=True)
        if evidence and command and command[0]=='build' and '--plan' not in command:
            invocation += 1
            destination = evidence / f'{invocation:02d}-{root.name}'
            destination.mkdir()
            if (root / 'dist').is_dir():
                shutil.copytree(root / 'dist', destination / 'dist')
            (destination / 'invocation.json').write_text(json.dumps({
                'command': ['oyzu', *command], 'expectedSuccess': success,
                'exitCode': result.returncode, 'stdout': result.stdout,
                'stderr': result.stderr,
            }, indent=2))
        if (result.returncode == 0) != success:
            logs = ""
            for p in (root / "dist/logs").glob("*"):
                logs += f"\n{p.name}:\n{p.read_text(errors='replace')[-12000:]}"
            raise AssertionError(f"{command}: {result.returncode}\n{result.stdout}\n{result.stderr}\n{logs}")
        return json.loads(result.stdout) if result.stdout.strip() else None

    selected = select_cases(args.suite)
    summary = {'suite': args.suite, 'status': 'failed', 'selectedChecks': [name for name, _ in selected],
               'completedChecks': [], 'verified': verified,
               'scope': 'implemented captured-build checks only; full authored scenario catalog remains pending'}
    try:
        with tempfile.TemporaryDirectory(prefix="oyzu-build-check-") as temporary:
            base = Path(temporary)
            for name, check in selected:
                summary['activeCheck'] = name
                check(ROOT, base, invoke, validate, source_files, verified)
                summary['completedChecks'].append(name)
            summary.pop('activeCheck', None)
            summary['status'] = 'succeeded'
    finally:
        if evidence:
            (evidence/'summary.json').write_text(json.dumps(summary,indent=2))
        print(json.dumps(summary,indent=2))


if __name__ == "__main__":
    main()
