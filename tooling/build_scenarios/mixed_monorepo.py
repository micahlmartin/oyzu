"""First complete EX-030 flow using the authored five-target project unchanged."""
import json
import shutil
import tarfile
import zipfile

import yaml

from .docker import image_contents


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'mixed-monorepo'
    shutil.copytree(root / 'examples/builds/mixed-monorepo/project', project)
    before = source_files(project)
    targets = {'api', 'web', 'command', 'image', 'chart'}
    stages = ('build', 'test', 'lint', 'format-check')
    tasks = invoke(project, 'run', 'list', '--json')
    for target in targets:
        for stage in stages:
            assert f'{target}:{stage}' in tasks
            # Docker's direct test command is still unavailable; its captured
            # OCI validation must nevertheless execute in the full build below.
            if (target, stage) != ('image', 'test'):
                assert tasks[f'{target}:{stage}']['build_stage']

    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded'
    assert source_files(project) == before, 'mixed build changed checked-in inputs'
    verify_outputs(project, manifest)
    assert invoke(project, 'inspect', 'dist')['planDigest'] == manifest['planDigest']
    verified.append('EX-030 first end-to-end flow: unchanged five-target configuration, grouped tasks, native build/test/lint/format checks, Python wheel/sdist/application, Node directory, Go binary, exact binary in OCI, ordered Helm chart/rendering, JUnit/coverage and inspected snapshot bundle; advanced negative cases and image-to-chart value binding remain separate work')


def verify_outputs(project, manifest):
    # The image's explicit platform propagates to its Go producer. Resolve the
    # actual variant by native ownership and platform, not its hashed ID text.
    command_target, = [t for t in manifest['targets']
                       if t['builder'] == 'go/app' and t['path'] == 'command']
    assert command_target['platform'] == {'os': 'linux', 'arch': 'amd64'}
    command_id = command_target['id']
    targets = {'api', 'web', command_id, 'image', 'chart'}
    stages = ('build', 'test', 'lint', 'format-check')
    assert {t['id'] for t in manifest['targets']} == targets
    actions = {a['id']: a for a in manifest['actions']}
    assert len(actions) == len(manifest['actions']), 'duplicate action execution'
    for target in targets:
        for stage in (*stages, 'package'):
            assert actions[f'{target}:{stage}']['status'] == 'succeeded'
        assert any(r['target'] == target and r['kind'] == 'test'
                   and r['summary']['passed'] > 0 for r in manifest['reports'])
    for target in ('api', 'web', command_id):
        assert any(r['target'] == target and r['kind'] == 'coverage'
                   and r['summary']['covered'] > 0 for r in manifest['reports'])

    artifacts = manifest['artifacts']
    assert {a['target'] for a in artifacts} == targets
    assert len({a['id'] for a in artifacts}) == len(artifacts)
    assert all('dev' in a['version'] and 'g' in a['version'] for a in artifacts)
    application, = [a for a in artifacts if a['target'] == 'api' and a['name'] == 'application']
    wheel, = [a for a in artifacts if a['target'] == 'api' and a['path'].endswith('.whl')]
    source, = [a for a in artifacts if a['target'] == 'api' and a['path'].endswith('.tar.gz')]
    with zipfile.ZipFile(project / 'dist' / application['path']) as archive:
        assert archive.read('api/__init__.py') == (project / 'api/api/__init__.py').read_bytes()
        assert '__main__.py' in archive.namelist()
    with zipfile.ZipFile(project / 'dist' / wheel['path']) as archive:
        metadata, = [p for p in archive.namelist() if p.endswith('.dist-info/METADATA')]
        assert f"Version: {wheel['version']}" in archive.read(metadata).decode()
    with tarfile.open(project / 'dist' / source['path']) as archive:
        assert any(p.endswith('/api/__init__.py') for p in archive.getnames())

    web, = [a for a in artifacts if a['target'] == 'web']
    assert web['kind'] == 'directory'
    assert (project / 'dist' / web['path'] / 'greeting.mjs').read_bytes() == (
        project / 'web/src/greeting.mjs').read_bytes()
    command, = [a for a in artifacts if a['target'] == command_id]
    image, = [a for a in artifacts if a['target'] == 'image']
    image_digest, config, files = image_contents(project / 'dist' / image['path'])
    assert image_digest == image['ociDigest']
    assert config['config']['Entrypoint'] == ['/server']
    assert files['server'] == (project / 'dist' / command['path']).read_bytes()
    receipt, = json.loads((project / 'dist/inputs/image.json').read_text())['inputs']
    assert receipt['artifact'] == command['id'] and receipt['digest'] == command['digest']
    assert receipt['path'] == 'bin/server'

    plan = json.loads((project / 'dist/plan.json').read_text())
    planned = {a['id']: a for a in plan['actions']}
    assert f'{command_id}:package' in planned['image:build']['dependsOn']
    chart_actions = [a for a in plan['actions'] if a['target'] == 'chart']
    assert any('image:package' in a['dependsOn'] for a in chart_actions)
    chart, = [a for a in artifacts if a['target'] == 'chart' and a['name'] == 'chart']
    rendered, = [a for a in artifacts if a['target'] == 'chart' and a['name'] == 'rendered']
    with tarfile.open(project / 'dist' / chart['path']) as archive:
        metadata = yaml.safe_load(archive.extractfile('greeting/Chart.yaml').read())
        assert metadata['version'] == chart['version']
    deployment, = yaml.safe_load_all((project / 'dist' / rendered['path']).read_text())
    assert deployment['kind'] == 'Deployment'
    # depends_on orders targets. It does not invent the still-undecided image
    # digest-to-chart-values binding; preserve the authored native values.
    assert deployment['spec']['template']['spec']['containers'][0]['image'] == (
        'registry.example.invalid/team/greeting:development')
