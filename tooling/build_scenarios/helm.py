"""Native Helm chart packaging and negative cases through the compiled CLI."""
import shutil
import tarfile
import yaml


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'helm-chart'
    shutil.copytree(root/'examples/builds/helm-chart/project', project)
    before = source_files(project)
    plan = invoke(project, 'build', '--plan')
    assert plan == invoke(project, 'build', '--plan'), 'Helm plans are not repeatable'
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert source_files(project) == before
    invoke(project, 'inspect', 'dist')
    artifact = next(a for a in manifest['artifacts'] if a['name']=='chart')
    assert '-dev.g' in artifact['version']
    with tarfile.open(project/'dist'/artifact['path']) as archive:
        chart = yaml.safe_load(archive.extractfile('greeting/Chart.yaml').read())
        assert chart['version'] == artifact['version']
        assert chart['appVersion'] == '0.1.0'
        lock_bytes = archive.extractfile('greeting/Chart.lock').read()
        lock = yaml.safe_load(lock_bytes)
        assert lock['dependencies'][0]['name'] == 'labels'
        assert lock['dependencies'][0]['version'] == '0.1.0'
        assert 'greeting/charts/labels/Chart.yaml' in archive.getnames()
    rendered = next(a for a in manifest['artifacts'] if a['name']=='rendered')
    document = list(yaml.safe_load_all((project/'dist'/rendered['path']).read_text()))
    assert document[0]['kind'] == 'Deployment'
    assert document[0]['spec']['replicas'] == 1
    for task in ['build', 'test', 'lint', 'package']:
        assert next(a for a in manifest['actions'] if a['id']==f'project:{task}')['status']=='succeeded'
    rebuilt = invoke(project, 'build')
    assert {a['name']:a['digest'] for a in rebuilt['artifacts']} == {a['name']:a['digest'] for a in manifest['artifacts']}
    verified.append('Helm: inferred nested chart, native local dependency closure, snapshot chart/rendered artifacts, lint/schema validation and repeatable output')

    # Source-owned locks are honored, and stale locks fail before packaging.
    (project/'chart/Chart.lock').write_bytes(lock_bytes)
    invoke(project, 'build')
    metadata = project/'chart/Chart.yaml'
    original = metadata.read_text()
    metadata.write_text(original.replace('version: 0.1.0\n    repository:', 'version: 0.2.0\n    repository:'))
    invoke(project, 'build', success=False)
    assert not validate(project/'dist')['artifacts']
    metadata.write_text(original)
    values = project/'chart/values.yaml'
    values.write_text(values.read_text().replace('replicaCount: 1', 'replicaCount: 0'))
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:test')['status']=='failed'
    assert source_files(project) == before
    verified.append('Helm: native stale lock rejection and invalid values fail without publishing artifacts or editing source')

    library = base/'helm-library'
    shutil.copytree(root/'examples/builds/helm-chart/project/labels', library)
    before = source_files(library)
    invoke(library, 'build')
    manifest = validate(library/'dist')
    assert len(manifest['artifacts']) == 1
    assert not any(a['id']=='project:test' for a in manifest['actions'])
    assert next(a for a in manifest['actions'] if a['id']=='project:lint')['status']=='succeeded'
    artifact = manifest['artifacts'][0]
    with tarfile.open(library/'dist'/artifact['path']) as archive:
        chart = yaml.safe_load(archive.extractfile('labels/Chart.yaml').read())
        assert chart['type'] == 'library'
        assert chart['version'] == artifact['version']
        assert 'labels/templates/_helpers.tpl' in archive.getnames()
    assert source_files(library) == before
    verified.append('Helm library chart is inferred, linted and packaged without treating it as an installable application')
