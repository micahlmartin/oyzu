"""EX-044's test-owned loopback service, including an actual unavailable endpoint."""
import shutil
import tarfile
import xml.etree.ElementTree as ET

from .node_fixtures import format_sources


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'service-test'
    shutil.copytree(root / 'examples/builds/service-test/project', project)
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    for stage in ('build', 'test', 'lint', 'format-check', 'package'):
        assert next(a for a in manifest['actions'] if a['id'] == f'project:{stage}')['status'] == 'succeeded'
    artifact, = manifest['artifacts']
    assert '-dev.g' in artifact['version']
    with tarfile.open(project / 'dist' / artifact['path']) as archive:
        assert archive.extractfile('package/dist/greeting.mjs').read() == (project / 'src/greeting.mjs').read_bytes()
    report, = [r for r in manifest['reports'] if r['kind'] == 'test']
    assert report['summary']['passed'] == 3 and report['summary']['failed'] == 0
    names = {case.attrib['name'] for case in ET.parse(project / 'dist' / report['path']).iter('testcase')}
    assert 'calls a test-owned loopback service' in names
    assert any(r['kind'] == 'coverage' and r['summary']['covered'] > 0 for r in manifest['reports'])
    invoke(project, 'inspect', 'dist')

    # Close the actual server, then call its former port. The test must execute
    # and fail; no simulated report, skipped assertion or unavailable-pass flag.
    test = project / 'test/service.test.mjs'
    original = test.read_text(encoding='utf-8')
    unavailable = original.replace(
        '  try {\n',
        '  const port = server.address().port;\n'
        '  await new Promise((resolve) => server.close(resolve));\n'
        '  try {\n', 1,
    ).replace('"http://127.0.0.1:" + server.address().port', '"http://127.0.0.1:" + port')
    assert unavailable != original
    test.write_text(unavailable, encoding='utf-8', newline='\n')
    format_sources(root, test)
    before_failure = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert failed['status'] == 'failed' and not failed['artifacts']
    assert source_files(project) == before_failure
    assert next(a for a in failed['actions'] if a['id'] == 'project:test')['status'] == 'failed'
    report, = [r for r in failed['reports'] if r['kind'] == 'test']
    assert report['summary']['passed'] == 2 and report['summary']['failed'] == 1
    service, = [case for case in ET.parse(project / 'dist' / report['path']).iter('testcase')
                if case.attrib['name'] == 'calls a test-owned loopback service']
    assert service.find('failure') is not None or service.find('error') is not None
    verified.append('EX-044 test-owned service: real loopback HTTP inside the isolated action, snapshot package, JUnit/coverage, unchanged source, closed endpoint fails the test and blocks artifacts; separately provisioned/external services remain pending')
