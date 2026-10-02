"""Real npm tarball preparation and offline native Dockerfile consumption."""
import json
import subprocess

from npm_context_fixtures import IMAGE, DOCKERFILE, create, lock_arguments
from .docker import image_contents


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'docker-npm-context'
    create(project)
    result = subprocess.run([
        'docker', 'run', '--rm', '--pull=never', '--mount',
        f'type=bind,source={project},target=/workspace', '--workdir', '/workspace',
        IMAGE, 'npm', *lock_arguments('/tmp/fixture-npm')],
        capture_output=True, text=True, timeout=300)
    assert result.returncode == 0, (result.stdout, result.stderr)
    (project / 'build.yaml').write_text('image:\n  uses: docker/image\n')
    (project / 'Dockerfile').write_text(DOCKERFILE)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list')
    assert all('image:' + name in tasks for name in ['build', 'test', 'lint', 'format-check'])
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    artifact, = manifest['artifacts']
    assert artifact['kind'] == 'oci-image' and '-dev.g' in artifact['version']
    _, _, files = image_contents(project / 'dist' / artifact['path'])
    assert json.loads(files['app/node_modules/is-odd/package.json'])['version'] == '3.0.1'
    assert json.loads(files['app/node_modules/is-number/package.json'])['version'] == '6.0.0'
    assert not any(name.startswith(('dependencies/', 'tmp/npm-cache/', 'app/node_modules/picocolors/')) for name in files)
    dependency = json.loads((project / 'dist/dependencies/image.json').read_text())
    context = dependency['extensions']['oyzu.dev/docker']['dependencyContext']
    assert context['provider'] == 'node/npm' and context['runtime']['reference'] == IMAGE
    assert context['snapshot']['manager']['id'] == 'npm'
    assert context['snapshot']['manager']['version'] == '11.11.0'
    assert context['snapshot']['extensions']['oyzu.dev/npm']['nodeVersion'].startswith('22.')
    assert {(p['name'], p['version'], p['purpose']) for p in dependency['packages']} == {
        ('is-odd', '3.0.1', 'runtime'), ('is-number', '6.0.0', 'runtime'), ('picocolors', '1.1.1', 'build')}
    plan = json.loads((project / 'dist/plan.json').read_text())
    assert all(a['network'] == 'none' for a in plan['actions'])
    assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    assert source_files(project) == before
    lock_path = project / 'package-lock.json'
    lock = json.loads(lock_path.read_text())
    lock['packages']['node_modules/is-odd']['integrity'] = 'sha512-' + 'A' * 86 + '=='
    lock_path.write_text(json.dumps(lock))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'integrity' in json.dumps(failed['diagnostics']).lower()
    verified.append('Docker npm dependency context: immutable consumer Node/npm runtime, native locked acquisition without lifecycle scripts, offline cache seeding and npm ci, runtime/dev evidence, omitted development installation, snapshot OCI/JUnit/quality, repeated identities, unchanged sources and rejected lock integrity; private registries remain pending')
