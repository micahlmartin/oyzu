"""Compiled CLI integration for native Vitest and captured npm dependencies."""
import json
import shutil
import tarfile


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'node-vitest'
    shutil.copytree(root / 'tooling/fixtures/vitest', project, ignore=shutil.ignore_patterns('node_modules'))
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert tasks['project:test']['argv'] == ['node', 'node_modules/vitest/vitest.mjs', 'run']
    assert tasks['project:test']['build_stage']
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and before == source_files(project)
    assert manifest['targets'][0]['extensions']['oyzu.dev/discovery']['test-framework']['selected'] == 'vitest'
    tests = next(r for r in manifest['reports'] if r['kind'] == 'test')
    assert tests['summary']['passed'] == 1 and tests['summary']['skipped'] == 2
    coverage = next(r for r in manifest['reports'] if r['kind'] == 'coverage')
    assert coverage['summary']['covered'] > 0
    assert 'src/greeting.js' in (project / 'dist' / coverage['path']).read_text()
    artifact = manifest['artifacts'][0]
    assert '-dev.g' in artifact['version']
    with tarfile.open(project / 'dist' / artifact['path']) as archive:
        assert json.load(archive.extractfile('package/package.json'))['version'] == artifact['version']
    record = json.loads((project / 'dist/dependencies/project.json').read_text())
    assert any(p['name'] == '@vitest/coverage-v8' and p['version'] == '5.0.3' for p in record['packages'])
    invoke(project, 'inspect', 'dist')

    package_file = project / 'package.json'
    package = json.loads(package_file.read_text())
    package['scripts'].update({'test': 'vitest run', 'pretest': 'node pretest.cjs'})
    package_file.write_text(json.dumps(package))
    (project / 'pretest.cjs').write_text("require('node:fs').appendFileSync('pretest-ran','pre\\n');\n")
    (project / 'test/lifecycle.test.js').write_text("""import {test, expect} from 'vitest';
import {readFileSync, existsSync} from 'node:fs';
test('native pretest ran once', () => expect(readFileSync('pretest-ran','utf8')).toBe('pre\\n'));
test('no acquisition broker', () => expect(existsSync('/broker')).toBe(false));
test('intentional failure', () => expect(1).toBe(2));
""")
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert source_files(project) == before and not failed['artifacts']
    assert all(r['status'] == 'collected' for r in failed['reports'])
    assert next(r for r in failed['reports'] if r['kind'] == 'test')['summary']['failed'] == 1
    invoke(project, 'inspect', 'dist')
    (project / 'test/lifecycle.test.js').unlink()
    (project / 'src/uncovered.js').write_text('export function unused() {\n  return 42;\n}\n')
    (project / 'vitest.config.mjs').write_text("export default {test:{coverage:{include:['src/**'],thresholds:{lines:100}}}};\n")
    invoke(project, 'build', success=False)
    threshold = validate(project / 'dist')
    assert not threshold['artifacts'] and all(r['status'] == 'collected' for r in threshold['reports'])
    assert next(r for r in threshold['reports'] if r['kind'] == 'test')['summary']['failed'] == 0
    measured = next(r for r in threshold['reports'] if r['kind'] == 'coverage')['summary']
    assert 0 < measured['covered'] < measured['total']
    invoke(project, 'inspect', 'dist')
    verified.append('Vitest: inferred/native-script tests, captured framework/coverage provider, automatic native JUnit/LCOV, snapshot package, lifecycle preservation, failed-test evidence and native coverage thresholds')
