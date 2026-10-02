"""Native Helm chart packaging and negative cases through the compiled CLI."""
import shutil
import subprocess
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
    assert any(r['kind']=='test' and r['summary']['passed']==1 for r in manifest['reports'])
    assert not any(r['kind']=='coverage' for r in manifest['reports'])
    assert manifest['targets'][0]['extensions']['oyzu.dev/coverage-applicability']['status']=='inapplicable'
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

    assert any(r['kind']=='test' and r['summary']['failed']==1 for r in failed['reports'])
    assert source_files(project) == before
    verified.append('Helm: native stale lock rejection and invalid values fail without publishing artifacts or editing source')

    unit = base/'helm-unittest'
    shutil.copytree(root/'examples/builds/helm-chart/project', unit)
    (unit/'chart/tests').mkdir()
    suite = unit/'chart/tests/deployment_test.yaml'
    shutil.copyfile(root/'examples/builds/helm-chart/variants/deployment_test.yaml',suite)
    (unit/'chart/.helmignore').write_text('tests/\n')
    listing = invoke(unit,'run','list','--json')
    assert listing['project:test']['argv'] == ['helm','unittest','--strict','chart']
    before = source_files(unit)
    invoke(unit,'build')
    manifest = validate(unit/'dist')
    assert source_files(unit)==before and manifest['status']=='succeeded'
    reports = [r for r in manifest['reports'] if r['kind']=='test']
    assert len(reports)==2 and all(r['summary']['passed']==1 for r in reports)
    assert not any(r['kind']=='coverage' for r in manifest['reports'])
    assert manifest['targets'][0]['extensions']['oyzu.dev/coverage-applicability']['status']=='inapplicable'
    assert manifest['targets'][0]['extensions']['oyzu.dev/discovery']['test-framework']['selected']=='helm-unittest'
    artifact = next(a for a in manifest['artifacts'] if a['name']=='chart')
    with tarfile.open(unit/'dist'/artifact['path']) as archive:
        assert '-dev.g' in yaml.safe_load(archive.extractfile('greeting/Chart.yaml'))['version']
        assert not any('/tests/' in name for name in archive.getnames())
    assert any(a['name']=='rendered' for a in manifest['artifacts'])
    suite.write_text(suite.read_text().replace('value: 1','value: 99'))
    before = source_files(unit)
    invoke(unit,'build',success=False)
    failed = validate(unit/'dist')
    assert source_files(unit)==before and not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:test')['status']=='failed'
    tests = [r for r in failed['reports'] if r['kind']=='test']
    assert len(tests)==2 and sum(r['summary']['failed'] for r in tests)==1
    assert sum(r['summary']['passed'] for r in tests)==1
    verified.append('Helm unittest: implicit native plugin selection, independent validation/suite JUnit, snapshot chart/rendered outputs, failed assertion blocks collection and unchanged checkout')
    suite.write_text((root/'examples/builds/helm-chart/variants/deployment_snapshot_test.yaml').read_text())
    before = source_files(unit)
    invoke(unit,'build',success=False)
    failed = validate(unit/'dist')
    assert source_files(unit)==before and not failed['artifacts']
    assert not (unit/'chart/tests/__snapshot__').exists()
    assert next(a for a in failed['actions'] if a['id']=='project:test')['status']=='failed'
    # Native unittest may report a pass after generating a missing expectation.
    # The adapter still rejects the task; retained native evidence is not rewritten.
    tests = [r for r in failed['reports'] if r['kind']=='test']
    assert len(tests)==2 and all(r['summary']['passed']==1 for r in tests)
    verified.append('Helm snapshot assertions: native baseline creation fails the captured task and blocks artifacts despite passing native JUnit, without modifying the checkout')
    library = base/'helm-library'
    shutil.copytree(root/'examples/builds/helm-chart/project/labels', library)
    before = source_files(library)
    invoke(library, 'build')
    manifest = validate(library/'dist')
    assert len(manifest['artifacts']) == 1
    assert next(a for a in manifest['actions'] if a['id']=='project:test')['status']=='succeeded'
    assert any(r['kind']=='test' and r['summary']['passed']==1 for r in manifest['reports'])
    assert manifest['targets'][0]['extensions']['oyzu.dev/coverage-applicability']['status']=='inapplicable'
    assert next(a for a in manifest['actions'] if a['id']=='project:lint')['status']=='succeeded'
    artifact = manifest['artifacts'][0]
    with tarfile.open(library/'dist'/artifact['path']) as archive:
        chart = yaml.safe_load(archive.extractfile('labels/Chart.yaml').read())
        assert chart['type'] == 'library'
        assert chart['version'] == artifact['version']
        assert 'labels/templates/_helpers.tpl' in archive.getnames()
    assert source_files(library) == before
    verified.append('Helm application and library charts retain native validation JUnit and explicit coverage inapplicability without cluster execution')

    # Replacing the test body cannot erase the builder-owned JUnit obligation.
    (library/'oyzu.toml').write_text('[tasks."project:test"]\nargv=["sh","-c","true"]\n')
    invoke(library, 'build', success=False)
    failed = validate(library/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:test')['status']=='failed'

    subcharts = base/'helm-subchart-only'
    shutil.copytree(root/'examples/builds/helm-chart/variants/subchart-only',subcharts)
    assert invoke(subcharts,'run','list','--json')['project:test']['argv']==['helm','unittest','--strict','.']
    before = source_files(subcharts)
    invoke(subcharts,'build')
    manifest = validate(subcharts/'dist')
    assert manifest['status']=='succeeded' and source_files(subcharts)==before
    tests = [r for r in manifest['reports'] if r['kind']=='test']
    assert len(tests)==2 and all(r['summary']['passed']==1 for r in tests)
    assert manifest['targets'][0]['extensions']['oyzu.dev/discovery']['test-framework']['selected']=='helm-unittest'
    assert len(manifest['artifacts'])==2
    artifact = next(a for a in manifest['artifacts'] if a['name']=='chart')
    assert '-dev.g' in artifact['version']
    with tarfile.open(subcharts/'dist'/artifact['path']) as archive:
        assert yaml.safe_load(archive.extractfile('parent/Chart.yaml'))['version']==artifact['version']
        assert 'parent/charts/child/templates/configmap.yaml' in archive.getnames()
    child_suite = subcharts/'charts/child/tests/configmap_test.yaml'
    child_suite.write_text(child_suite.read_text().replace('value: "42"','value: "99"'))
    before = source_files(subcharts)
    invoke(subcharts,'build',success=False)
    failed = validate(subcharts/'dist')
    assert not failed['artifacts'] and source_files(subcharts)==before
    assert next(a for a in failed['actions'] if a['id']=='project:test')['status']=='failed'
    assert any(r['kind']=='test' and r['summary']['failed']==1 for r in failed['reports'])
    verified.append('Helm subchart-only suites: static native plugin selection, snapshot chart/render artifacts, native assertion JUnit and artifact rejection after a subchart failure')

    packed = base/'helm-packaged-only'
    shutil.copytree(root/'examples/builds/helm-chart/variants/packaged-only',packed)
    listing = invoke(packed,'run','list','--json')
    assert listing['project:test']['argv']==['helm','unittest','--strict','.']
    before = source_files(packed)
    invoke(packed,'build')
    manifest = validate(packed/'dist')
    assert manifest['status']=='succeeded' and source_files(packed)==before
    reports = [r for r in manifest['reports'] if r['kind']=='test']
    assert len(reports)==2 and all(r['summary']['passed']==1 for r in reports)
    assert len(manifest['artifacts'])==2
    artifact = next(a for a in manifest['artifacts'] if a['name']=='chart')
    assert '-dev.g' in artifact['version']
    with tarfile.open(packed/'dist'/artifact['path']) as archive:
        assert 'packaged-parent/charts/child/templates/configmap.yaml' in archive.getnames()
    repeated = invoke(packed,'build')
    assert {a['id']:a['digest'] for a in repeated['artifacts']}=={a['id']:a['digest'] for a in manifest['artifacts']}
    # Native fixture creation, rather than hand-authored chart archive bytes.
    child = base/'helm-bad-packaged-child'
    shutil.copytree(root/'examples/builds/helm-chart/variants/subchart-only/charts/child',child)
    suite = child/'tests/configmap_test.yaml'
    suite.write_text(suite.read_text().replace('value: "42"','value: "99"'))
    subprocess.run(['docker','run','--rm','--network=none','--entrypoint','helm',
                    '--mount',f'type=bind,source={child},target=/chart,readonly',
                    '--mount',f'type=bind,source={packed/"charts"},target=/output',
                    'oyzu-toolchain/helm:3.22.0','package','/chart','--destination','/output'],check=True)
    before = source_files(packed)
    invoke(packed,'build',success=False)
    failed = validate(packed/'dist')
    assert not failed['artifacts'] and source_files(packed)==before
    assert next(a for a in failed['actions'] if a['id']=='project:test')['status']=='failed'
    assert any(r['kind']=='test' and r['summary']['failed']==1 for r in failed['reports'])
    verified.append('Packaged Helm dependencies: static archive evidence, native assertions, unchanged input archives, repeatable chart snapshots and failed-suite artifact rejection')
