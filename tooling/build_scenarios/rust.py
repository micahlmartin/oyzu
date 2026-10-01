"""Native Cargo checks shared by the compiled-CLI scenario harness."""
import json
import shutil
import subprocess
import tarfile
import tomllib
import xml.etree.ElementTree as ET

from jsonschema import Draft202012Validator


IMAGE = 'oyzu-toolchain/rust:1.94.0-nextest0.9.146-llvmcov0.9.1'


def binary(manifest):
    return next(a for a in manifest['artifacts'] if a['mediaType']=='application/octet-stream')


def archives(project, manifest, count):
    packages = [a for a in manifest['artifacts'] if a['name'].startswith('crate.')]
    assert len(packages) == count
    for artifact in packages:
        name = artifact['name'].removeprefix('crate.')
        with tarfile.open(project/'dist'/artifact['path']) as archive:
            prefix = f"{name}-{artifact['version']}/"
            metadata = tomllib.loads(archive.extractfile(prefix+'Cargo.toml').read().decode())
            assert metadata['package']['name'] == name
            assert metadata['package']['version'] == artifact['version']
            assert prefix+'Cargo.lock' in archive.getnames()
            assert any(n.startswith(prefix+'src/') for n in archive.getnames())
            assert all('.oyzu-build/' not in n for n in archive.getnames())
            for dependency in metadata.get('dependencies', {}).values():
                assert '-dev.g' in dependency['version']
                assert 'path' not in dependency


def application_coverage(project, manifest, source, expression):
    report = next(r for r in manifest['reports'] if r['kind']=='coverage')
    assert report['summary']['covered'] > 0
    line = next(i for i, text in enumerate((project/source).read_text().splitlines(), 1) if expression in text)
    document = ET.parse(project/'dist'/report['path'])
    classes = [c for c in document.findall('.//class') if c.attrib['filename'].replace('\\','/').endswith(source)]
    assert classes, f'coverage did not include {source}'
    assert any(int(n.attrib['number'])==line and int(n.attrib['hits'])>0 for c in classes for n in c.findall('./lines/line')), f'coverage did not measure executed application line {source}:{line}'


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
        archives(project, manifest, 3 if example=='rust-workspace' else 1)
        artifact = binary(manifest)
        assert '-dev.g' in artifact['version']
        assert run_binary(project, artifact) == 'Hello, Oyzu!'
        assert next(r for r in manifest['reports'] if r['kind']=='test')['summary']['passed'] > 0
        application_coverage(project, manifest, 'core/src/lib.rs' if example=='rust-workspace' else 'src/main.rs', 'MESSAGE.to_owned()' if example=='rust-workspace' else 'format!("Hello, {name}!")')
        for stage in ['build', 'test', 'lint', 'format-check', 'archive', 'package']:
            assert next(a for a in manifest['actions'] if a['id']==f'project:{stage}')['status']=='succeeded'
        dependency = json.loads((project/'dist/dependencies/project.json').read_text())
        Draft202012Validator(schema).validate(dependency)
        workspace = dependency['extensions']['oyzu.dev/cargo-workspace']
        assert len(workspace['projected']['packages']) == (3 if example=='rust-workspace' else 1)
        assert all('-dev.g' in p['version'] for p in workspace['projected']['packages'])
        rebuilt = invoke(project, 'build')
        assert {a['name']:a['digest'] for a in rebuilt['artifacts']} == {a['name']:a['digest'] for a in manifest['artifacts']}, 'Cargo artifacts were not repeatable'
        verified.append(f'{example}: native workspace resolution, snapshot versions, offline build/test/clippy/fmt, JUnit, delivered binary, verified crate archives and repeatability')

    project = base/'rust-library'
    shutil.copytree(root/'examples/builds/rust-workspace/project/core', project)
    (project/'Cargo.lock').write_text('version = 4\n\n[[package]]\nname = "example-core"\nversion = "0.1.0"\n')
    before = source_files(project)
    invoke(project, 'build')
    library_manifest = validate(project/'dist')
    archives(project, library_manifest, 1)
    assert len(library_manifest['artifacts']) == 1
    assert source_files(project) == before
    assert next(r for r in library_manifest['reports'] if r['kind']=='test')['summary']['passed'] > 0
    application_coverage(project, library_manifest, 'src/lib.rs', 'MESSAGE.to_owned()')
    verified.append('Cargo library-only project produces a native verified snapshot crate without a binary')

    project = base/'rust-workspace'
    previous = binary(validate(project/'dist'))
    (project/'core/message.txt').write_text('Hello, changed input!\n')
    library = project/'core/src/lib.rs'
    library.write_text(library.read_text().replace('Hello, Oyzu!', 'Hello, changed input!'))
    invoke(project, 'build')
    changed = binary(validate(project/'dist'))
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
    application_coverage(project, failed, 'src/main.rs', 'format!("Hello, {name}!")')
    assert next(a for a in failed['actions'] if a['id']=='project:package')['status']=='blocked'
    source.write_text(good.replace('fn main() {', 'fn main(){'))
    unformatted = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:format-check')['status']=='failed'
    assert source_files(project) == unformatted
    verified.append('Cargo failing tests retain native JUnit; formatting failure blocks packaging without changing source')
