"""EX-031's zero-config generation-to-tested-package flow through the real CLI."""
import json
import shutil
import tarfile
import xml.etree.ElementTree as ET


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'generated-source'
    shutil.copytree(root / 'examples/builds/generated-source/project', project)
    before = source_files(project)
    assert not (project / 'build.yaml').exists()
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    assert not (project / 'generated').exists(), 'generation escaped the private build workspace'
    assert len([a for a in manifest['actions'] if a['id'] == 'project:build']) == 1
    for stage in ('build', 'test', 'lint', 'format-check', 'package'):
        assert next(a for a in manifest['actions'] if a['id'] == f'project:{stage}')['status'] == 'succeeded'
    artifact, = manifest['artifacts']
    assert '-dev.g' in artifact['version']
    with tarfile.open(project / 'dist' / artifact['path']) as archive:
        assert json.load(archive.extractfile('package/package.json'))['version'] == artifact['version']
        assert archive.extractfile('package/generated/message.mjs').read() == b'export const message = "Hello, Oyzu!";\n'
        assert archive.extractfile('package/schema.json').read() == (project / 'schema.json').read_bytes()
    report, = [r for r in manifest['reports'] if r['kind'] == 'test']
    assert report['summary']['passed'] == 2 and report['summary']['failed'] == 0
    names = {case.attrib['name'] for case in ET.parse(project / 'dist' / report['path']).iter('testcase')}
    assert {'api consumes generated data', 'web consumes generated data'} <= names
    assert any(r['kind'] == 'coverage' and r['summary']['covered'] > 0 for r in manifest['reports'])
    invoke(project, 'inspect', 'dist')

    # Both real consumers must observe changed generated content. This proves
    # regeneration, not selective cache invalidation (which remains separate).
    (project / 'schema.json').write_text('{"greeting":"Changed schema"}\n', encoding='utf-8')
    changed = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert failed['status'] == 'failed' and not failed['artifacts']
    assert source_files(project) == changed and not (project / 'generated').exists()
    assert next(a for a in failed['actions'] if a['id'] == 'project:build')['status'] == 'succeeded'
    assert next(a for a in failed['actions'] if a['id'] == 'project:test')['status'] == 'failed'
    assert next(r for r in failed['reports'] if r['kind'] == 'test')['summary']['failed'] == 2
    verified.append('EX-031 first flow: zero-config native generation, both consumers tested, generated module in inspected snapshot package, JUnit/coverage, unchanged checkout and changed schema observed by both tests; action-granular caching remains pending')
