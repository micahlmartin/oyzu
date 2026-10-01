"""Real inferred Jest builds with broker-captured npm dependencies."""
import json
import shutil
import tarfile


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'node-jest'
    shutil.copytree(root / 'tooling/fixtures/jest', project, ignore=shutil.ignore_patterns('node_modules'))
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert tasks['project:test']['argv'] == ['node', 'node_modules/jest/bin/jest.js', '--ci']
    assert tasks['project:test']['build_stage']
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    assert manifest['targets'][0]['extensions']['oyzu.dev/discovery']['test-framework']['selected'] == 'jest'
    report = next(r for r in manifest['reports'] if r['kind'] == 'test')
    assert report['summary']['total'] == 3
    assert report['summary']['passed'] == 1 and report['summary']['skipped'] == 2
    coverage = next(r for r in manifest['reports'] if r['kind'] == 'coverage')
    assert coverage['summary']['covered'] > 0
    assert 'src/greeting.js' in (project / 'dist' / coverage['path']).read_text()
    artifact = manifest['artifacts'][0]
    assert '-dev.g' in artifact['version']
    with tarfile.open(project / 'dist' / artifact['path']) as archive:
        assert json.load(archive.extractfile('package/package.json'))['version'] == artifact['version']
        assert 'package/src/greeting.js' in archive.getnames()
    dependency = json.loads((project / 'dist/dependencies/project.json').read_text())
    assert any(p['name'] == 'jest' and p['version'] == '29.7.0' for p in dependency['packages'])
    invoke(project, 'inspect', 'dist')

    # Recognized native scripts retain npm lifecycle behavior while receiving
    # automatic reports. Failed tests must retain both reports and block packing.
    package_path = project / 'package.json'
    package = json.loads(package_path.read_text())
    package['scripts'].update({'test': 'jest --ci', 'pretest': 'node pretest.cjs'})
    package_path.write_text(json.dumps(package))
    (project / 'pretest.cjs').write_text("require('node:fs').appendFileSync('pretest-ran','pre\\n');\n")
    (project / 'test/lifecycle.test.js').write_text("""test('native pretest ran exactly once',()=>{
  expect(require('node:fs').readFileSync('pretest-ran','utf8')).toBe('pre\\n');
});
test('intentional failure',()=>{expect(1).toBe(2)});
""")
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert source_files(project) == before and not failed['artifacts']
    assert all(r['status'] == 'collected' for r in failed['reports'])
    report = next(r for r in failed['reports'] if r['kind'] == 'test')
    assert report['summary']['failed'] == 1 and report['summary']['passed'] == 2
    assert next(a for a in failed['actions'] if a['id'] == 'project:test')['status'] == 'failed'
    assert next(a for a in failed['actions'] if a['id'] == 'project:package')['status'] == 'blocked'
    invoke(project, 'inspect', 'dist')
    verified.append('Jest: inferred runner, captured registry closure, native lifecycle scripts, automatic JUnit/LCOV, snapshot package, retained failed evidence')
