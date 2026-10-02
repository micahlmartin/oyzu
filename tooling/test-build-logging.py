"""Real CLI logging proof: parallel Rust snapshots, live streams and failed stages."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import threading

parser = argparse.ArgumentParser()
parser.add_argument('--cli', type=Path, required=True)
parser.add_argument('--evidence-dir', type=Path, required=True)
args = parser.parse_args()
cli = args.cli.resolve()
root = Path(__file__).resolve().parents[1]
evidence = args.evidence_dir.resolve()
evidence.mkdir(parents=True, exist_ok=True)
project = evidence / 'project'
project.mkdir()
for target in ['alpha', 'beta']:
    shutil.copytree(root / 'examples/builds/rust-app/project', project / target)
(project / 'build.yaml').write_text('alpha: {uses: rust/app, path: alpha}\nbeta: {uses: rust/app, path: beta}\n', encoding='utf-8', newline='\n')
configuration = '[build]\njobs=2\n'
for target in ['alpha', 'beta']:
    command = ['python3', '-c', f'import sys,time; print("{target} live stdout", flush=True); print("{target} live stderr", file=sys.stderr, flush=True); time.sleep(3); print("{target} final fragment",end="",flush=True)']
    configuration += f'[tasks."{target}:pre_build"]\nargv = {json.dumps(command)}\n'
(project / 'oyzu.toml').write_text(configuration, encoding='utf-8', newline='\n')


def run(name, json_mode=False):
    command = [str(cli), '--root', str(project), *(['--json'] if json_mode else []), 'build']
    (evidence / f'{name}.command.json').write_text(json.dumps(command, indent=2)+'\n', encoding='utf-8', newline='\n')
    with (evidence / f'{name}.stdout').open('w', encoding='utf-8', newline='\n') as out:
        env = os.environ.copy()
        env.update(GITHUB_ACTIONS='true', GITHUB_STEP_SUMMARY=str(evidence/f'{name}.summary.md'))
        process = subprocess.Popen(command, stdout=out, stderr=subprocess.PIPE, text=True, encoding='utf-8', env=env)
        lines = []
        live = set()
        def consume():
            with (evidence / f'{name}.stderr').open('w', encoding='utf-8', newline='\n') as err:
                for line in process.stderr:
                    lines.append(line)
                    err.write(line)
                    err.flush()
                    for target in ['alpha', 'beta']:
                        if json_mode:
                            event = json.loads(line)['event']
                            output = event.get('type') == 'output' and event.get('text') == f'{target} live stdout'
                        else:
                            output = f'stdout | {target} live stdout' in line
                        if output and process.poll() is None:
                            live.add(target)
        reader = threading.Thread(target=consume)
        reader.start()
        try:
            code = process.wait(timeout=600)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            reader.join(timeout=10)
    assert code in [0, 1], (code, ''.join(lines))
    journal = [json.loads(line) for line in (project/'dist/logs/events.jsonl').read_text(encoding='utf-8').splitlines()]
    assert all(e['sequence'] < f['sequence'] for e, f in zip(journal, journal[1:]))
    manifest = json.loads((project/'dist/manifest.json').read_text(encoding='utf-8'))
    shutil.copytree(project/'dist', evidence/f'{name}-bundle')
    subprocess.run([str(cli), '--root', str(project), 'inspect', 'dist'], check=True, capture_output=True, text=True)
    return code, ''.join(lines), journal, manifest, live


code, text, events, manifest, live = run('success')
assert code == 0 and manifest['status'] == 'succeeded', text
assert live == {'alpha', 'beta'}, live
assert len(manifest['artifacts']) == 4
assert 'Oyzu build: succeeded' in (evidence/'success.stdout').read_text(encoding='utf-8')
assert '\x1b' not in text
assert 'BUILD PLAN' in text and 'START' in text and 'PASS' in text
assert text.count('::group::') == text.count('::endgroup::') == 3
assert text.rfind('::endgroup::') < text.index('PHASE  Execute')
assert '2 passed' in (evidence/'success.summary.md').read_text(encoding='utf-8')
inventory = next(e['event']['plan'] for e in events if e['event']['type']=='plan')
assert all(set(a) == {'id', 'target', 'dependsOn'} for a in inventory['actions'])
for target in ['alpha','beta']:
    scope = f'{target}:pre_build'
    output = [e for e in events if e['scope'] == scope and e['event']['type'] == 'output']
    assert any(e['event']['stream']=='stdout' and e['event']['text']==f'{target} live stdout' for e in output)
    assert any(e['event']['stream']=='stderr' and e['event']['text']==f'{target} live stderr' for e in output)
    assert any(e['event']['text']==f'{target} final fragment' for e in output)
    first = next(e for e in output if e['event']['text']==f'{target} live stdout')
    finished = next(e for e in events if e['scope']==scope and e['event'].get('status')=='command-succeeded')
    assert first['elapsedMs'] < finished['elapsedMs'] - 1000, (first, finished)
intervals = []
for target in ['alpha','beta']:
    scoped = [e for e in events if e['scope']==f'{target}:pre_build']
    start = next(e['elapsedMs'] for e in scoped if e['event']['type']=='command')
    end = next(e['elapsedMs'] for e in scoped if e['event'].get('status')=='command-succeeded')
    intervals.append((start,end))
assert max(i[0] for i in intervals) < min(i[1] for i in intervals), intervals

configuration += '[tasks."alpha:pre_test"]\nargv = ["python3", "-c", "import sys; print(\'deliberate logging failure\', file=sys.stderr); sys.exit(7)"]\n'
(project/'oyzu.toml').write_text(configuration, encoding='utf-8', newline='\n')
code, text, events, manifest, live = run('failure', json_mode=True)
assert code == 1 and manifest['status']=='failed', text
assert json.loads((evidence/'failure.stdout').read_text(encoding='utf-8')) == manifest
console = [json.loads(line) for line in text.splitlines()]
assert any(e['scope']=='alpha:pre_test' and e['event'].get('text')=='deliberate logging failure' for e in console)
assert any(a['id']=='alpha:pre_test' and a['exitCode']==7 for a in manifest['actions'])
assert any(a['id']=='alpha:package' and a['status']=='blocked' for a in manifest['actions'])
assert any(e['event'].get('status')=='failed' for e in events)
print(f'Live parallel stdout/stderr, partial lines, exact commands, four snapshot artifacts, JSON failure/blocked events and inspected bundles passed. Evidence: {evidence}')
