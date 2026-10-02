"""Native Go workspace/cgo builds and selected multi-binary container assembly."""
import hashlib
import json
import shutil
import subprocess
from jsonschema import Draft202012Validator
from .docker import image_contents


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'go-workspace-cgo'
    shutil.copytree(root / 'examples/builds/go-workspace-cgo/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all(tasks[f'project:{name}']['build_stage'] for name in ['build', 'test', 'lint', 'format-check'])
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    artifact, = manifest['artifacts']
    assert manifest['status'] == 'succeeded' and '-dev.g' in artifact['version']
    assert source_files(project) == before
    assert any(r['kind'] == 'test' and r['summary']['passed'] == 1 for r in manifest['reports'])
    assert any(r['kind'] == 'coverage' and r['summary']['covered'] > 0 for r in manifest['reports'])
    dependency = json.loads((project / 'dist/dependencies/project.json').read_text())
    metadata = dependency['extensions']['oyzu.dev/go-metadata']
    assert metadata['modules'] == ['cmd', 'math'] and metadata['cgo']
    assert metadata['compiler'] and metadata['compilerTarget']
    path = project / 'dist' / artifact['path']
    result = subprocess.run(['docker', 'run', '--rm', '--network=none', '--mount',
                             f'type=bind,source={path},target=/app,readonly',
                             '--entrypoint', '/app', 'golang:1.24-bookworm'], capture_output=True, text=True)
    assert result.returncode == 0 and result.stdout.strip() == '5', (result.stdout, result.stderr)
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    (project / 'oyzu.toml').write_text('[env]\nCGO_ENABLED="0"\n')
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert any('CGO_ENABLED must remain 1' in d['message'] for d in failed['diagnostics'])
    (project / 'oyzu.toml').unlink()
    (project / 'math/failure_test.go').write_text('package math\n\nimport "testing"\n\nfunc TestFailure(t *testing.T) { t.Fatal("expected failure") }\n')
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['artifacts']
    assert any(r['kind'] == 'test' and r['summary']['failed'] == 1 for r in failed['reports'])
    verified.append('EX-017 build: native workspace ownership, cgo compiler/ABI evidence, repeatable snapshot binary, JUnit/coverage, disabled-cgo admission and failed-test retention')

    project = base / 'materialize-selected-artifacts'
    shutil.copytree(root / 'examples/builds/materialize-selected-artifacts/project', project)
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    binaries = {a['name']: a for a in manifest['artifacts'] if a['target'] == 'tools'}
    assert set(binaries) == {'server', 'migrate'}
    image, = [a for a in manifest['artifacts'] if a['target'] == 'image']
    _, _, files = image_contents(project / 'dist' / image['path'])
    for name, artifact in binaries.items():
        assert '-dev.g' in artifact['version']
        assert hashlib.sha256(files[name]).hexdigest() == artifact['digest'].removeprefix('sha256:')
    configuration = (project / 'build.yaml').read_text()
    # Preserve native indentation while removing only the selected artifact key.
    (project / 'build.yaml').write_text('\n'.join(line for line in configuration.splitlines() if 'artifact: server' not in line) + '\n')
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert any('explicit artifact selection' in d['message'] for d in failed['diagnostics'])
    (project / 'build.yaml').write_text(configuration.replace('linux/amd64', 'linux/arm64'))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and any('required platform' in d['message'] for d in failed['diagnostics'])
    verified.append('EX-049 build: named native binaries, versioned outputs, digest-preserving Docker assembly, ambiguous selector and incompatible execution platform rejection')
    verify_registry(root, base, invoke, validate, source_files, verified)


def verify_registry(root, base, invoke, validate, source_files, verified):
    project = base/'go-registry'
    shutil.copytree(root/'tooling/fixtures/go-registry', project)
    (project/'oyzu.toml').write_text('''[tasks."project:pre_test"]
argv = ["sh", "-ec", "test ! -e /broker; test $(go env GOPROXY) = off; if touch /dependencies/modules/mutation 2>/dev/null; then exit 1; fi"]
''')
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    artifact, = manifest['artifacts']
    assert any(r['kind'] == 'test' and r['summary']['passed'] > 0 for r in manifest['reports'])
    dependency = json.loads((project/'dist/dependencies/project.json').read_text())
    Draft202012Validator(json.loads((root/'docs/contracts/v1alpha1/dependencies.schema.json').read_text())).validate(dependency)
    package, = dependency['packages']
    assert package['name'] == 'github.com/google/uuid' and package['version'] == 'v1.6.0'
    assert package['sourceId'] == 'go-public' and package['verification'] == 'digest-only'
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    original = (project/'go.sum').read_text()
    (project/'go.sum').write_text(original.replace('NIvaJDMOsjHA8n1jAhLSgzrAzy1Hgr+hNrb57e+94F0=', 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA='))
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert any('checksum mismatch' in d['message'] for d in failed['diagnostics'])
    (project/'go.sum').unlink()
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert any('missing go.sum entry' in d['message'] for d in failed['diagnostics'])
    verified.append('Go registry: scoped native module capture, locked checksums, immutable offline replay without a broker, snapshot identity and checksum failures before actions')
