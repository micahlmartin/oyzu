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


def archives(project, manifest, count, external=()):
    packages = [a for a in manifest['artifacts'] if a['name'].startswith('crate-')]
    assert len(packages) == count
    for artifact in packages:
        name = (project/'dist'/artifact['path']).name.removesuffix(f"-{artifact['version']}.crate")
        with tarfile.open(project/'dist'/artifact['path']) as archive:
            prefix = f"{name}-{artifact['version']}/"
            metadata = tomllib.loads(archive.extractfile(prefix+'Cargo.toml').read().decode())
            assert metadata['package']['name'] == name
            assert metadata['package']['version'] == artifact['version']
            assert prefix+'Cargo.lock' in archive.getnames()
            assert any(n.startswith(prefix+'src/') for n in archive.getnames())
            assert all('.oyzu-build/' not in n for n in archive.getnames())
            for name, dependency in metadata.get('dependencies', {}).items():
                if name not in external:
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
    project = base/'rust-registry'
    shutil.copytree(root/'examples/builds/rust-app/variants/registry', project)
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert source_files(project) == before
    archives(project, manifest, 1, external=('itoa',))
    assert len(manifest['artifacts']) == 2, 'dependency crates are inputs, not workspace artifacts'
    assert run_binary(project, binary(manifest)) == 'Answer: 42'
    dependency = json.loads((project/'dist/dependencies/project.json').read_text())
    Draft202012Validator(schema).validate(dependency)
    assert [(p['name'],p['version']) for p in dependency['packages']] == [('itoa','1.0.15')]
    assert dependency['packages'][0]['digest'] == 'sha256:4a5f13b858c8d314ee3e8f639011f7ccefe71f97f96e50151fb991f267928e2c'
    assert next(r for r in manifest['reports'] if r['kind']=='test')['summary']['passed'] > 0
    application_coverage(project, manifest, 'src/main.rs', 'itoa::Buffer::new()')
    for stage in ['build', 'test', 'lint', 'format-check', 'archive', 'package']:
        assert next(a for a in manifest['actions'] if a['id']==f'project:{stage}')['status']=='succeeded'
    rebuilt = invoke(project, 'build')
    assert {a['name']:a['digest'] for a in rebuilt['artifacts']} == {a['name']:a['digest'] for a in manifest['artifacts']}
    lock = project/'Cargo.lock'
    lock.write_text(lock.read_text().replace('4a5f13b858c8d314ee3e8f639011f7ccefe71f97f96e50151fb991f267928e2c', '0'*64))
    invoke(project, 'build', success=False)
    assert not validate(project/'dist')['artifacts']
    verified.append('Cargo crates.io acquisition verifies locked checksums; offline native builds retain snapshot artifacts/JUnit/coverage and reject altered locks')

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

    # Native required-features controls artifact membership, not just execution.
    project = base/'rust-feature-gates'
    shutil.copytree(root/'examples/builds/rust-app/project', project)
    variants = root/'examples/builds/rust-app/variants'
    shutil.copyfile(variants/'features.Cargo.toml', project/'Cargo.toml')
    shutil.copyfile(variants/'extra.rs', project/'src/extra.rs')
    feature_manifest = project/'Cargo.toml'
    disabled = feature_manifest.read_text()
    for enabled in [False, True]:
        feature_manifest.write_text(disabled.replace('extra = []', 'extra = []\ndefault = ["extra"]') if enabled else disabled)
        before = source_files(project)
        invoke(project, 'build')
        manifest = validate(project/'dist')
        assert source_files(project) == before
        archives(project, manifest, 1)
        binaries = [a for a in manifest['artifacts'] if a['mediaType']=='application/octet-stream']
        assert len(binaries) == (2 if enabled else 1)
        assert {run_binary(project, a) for a in binaries} == ({'Hello, Oyzu!', 'Optional command'} if enabled else {'Hello, Oyzu!'})
        assert all('-dev.g' in a['version'] for a in manifest['artifacts'])
        assert next(r for r in manifest['reports'] if r['kind']=='test')['summary']['passed'] > 0
        application_coverage(project, manifest, 'src/main.rs', 'format!("Hello, {name}!")')
    feature_manifest.write_text(disabled)
    invoke(project, 'build')
    assert len([a for a in validate(project/'dist')['artifacts'] if a['mediaType']=='application/octet-stream']) == 1
    (project/'src/main.rs').write_text('compile_error!("native compile failure");\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:build')['status']=='failed'
    verified.append('Cargo native feature gates select executable artifacts; compiler-message collection retains executable identity and never reuses disabled or failed outputs')

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
