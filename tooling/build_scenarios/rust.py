"""Native Cargo checks shared by the compiled-CLI scenario harness."""
import json
import shutil
import subprocess

from jsonschema import Draft202012Validator


IMAGE = 'oyzu-toolchain/rust:1.94.0-nextest0.9.146'


def run_binary(project, artifact):
    path = project / 'dist' / artifact['path']
    result = subprocess.run([
        'docker', 'run', '--rm', '--network=none', '--read-only',
        '--mount', f'type=bind,source={path},target=/app,readonly',
        '--entrypoint', '/app', IMAGE,
    ], capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, (result.stdout, result.stderr)
    return result.stdout.strip()


def verify(root, base, invoke, validate, source_files, verified):
    schema = json.loads((root/'docs/contracts/v1alpha1/dependencies.schema.json').read_text())
    for example in ['rust-app', 'rust-workspace']:
        project = base / example
        shutil.copytree(root/'examples/builds'/example/'project', project)
        before = source_files(project)
        plan = invoke(project, 'build', '--plan')
        assert plan == invoke(project, 'build', '--plan'), 'Cargo plan changed for identical inputs'
        invoke(project, 'build')
        manifest = validate(project/'dist')
        assert before == source_files(project)
        invoke(project, 'inspect', 'dist')
        assert len(manifest['artifacts']) == 1
        artifact = manifest['artifacts'][0]
        assert '-dev.g' in artifact['version']
        assert run_binary(project, artifact) == 'Hello, Oyzu!'
        assert next(r for r in manifest['reports'] if r['kind']=='test')['summary']['passed'] > 0
        for stage in ['build', 'test', 'lint', 'format-check', 'package']:
            assert next(a for a in manifest['actions'] if a['id']==f'project:{stage}')['status']=='succeeded'
        dependency = json.loads((project/'dist/dependencies/project.json').read_text())
        Draft202012Validator(schema).validate(dependency)
        workspace = dependency['extensions']['oyzu.dev/cargo-workspace']
        assert len(workspace['projected']['packages']) == (3 if example=='rust-workspace' else 1)
        assert all('-dev.g' in p['version'] for p in workspace['projected']['packages'])
        rebuilt = invoke(project, 'build')
        assert rebuilt['artifacts'][0]['digest'] == artifact['digest'], 'Cargo binary was not repeatable'
        verified.append(f'{example}: native workspace resolution, snapshot versions, offline build/test/clippy/fmt, JUnit, delivered binary and repeatability')

    project = base/'rust-workspace'
    previous = validate(project/'dist')['artifacts'][0]
    (project/'core/message.txt').write_text('Hello, changed input!\n')
    library = project/'core/src/lib.rs'
    library.write_text(library.read_text().replace('Hello, Oyzu!', 'Hello, changed input!'))
    invoke(project, 'build')
    changed = validate(project/'dist')['artifacts'][0]
    assert changed['digest'] != previous['digest']
    assert run_binary(project, changed) == 'Hello, changed input!'
    verified.append('Cargo build-script input changes alter the delivered binary; workspace proc macro executes offline')

    project = base/'rust-app'
    source = project/'src/main.rs'
    good = source.read_text()
    source.write_text(good.replace('assert_eq!(greeting("Oyzu"), "Hello, Oyzu!")', 'assert_eq!(greeting("Oyzu"), "intentional failure")'))
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(r for r in failed['reports'] if r['kind']=='test')['summary']['failed'] > 0
    assert next(a for a in failed['actions'] if a['id']=='project:package')['status']=='blocked'
    source.write_text(good.replace('fn main() {', 'fn main(){'))
    unformatted = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:format-check')['status']=='failed'
    assert source_files(project) == unformatted
    verified.append('Cargo failing tests retain native JUnit; formatting failure blocks packaging without changing source')
