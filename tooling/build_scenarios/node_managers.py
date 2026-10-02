"""EX-019 native manager builds through the compiled CLI and isolated executor."""
import json
import shutil
import tarfile
from jsonschema import Draft202012Validator


def verify(root, base, invoke, validate, source_files, verified):
    schema = json.loads((root / 'docs/contracts/v1alpha1/dependencies.schema.json').read_text())
    for manager, version, fixture, dependency_count in [
        ('pnpm', '10.11.0', 'examples/builds/node-managers/pnpm', 0),
        ('yarn', '1.22.22', 'examples/builds/node-managers/yarn', 0),
        ('pnpm', '10.11.0', 'tooling/fixtures/pnpm-registry', 2),
        ('yarn', '1.22.22', 'tooling/fixtures/yarn-registry', 3),
    ]:
        project = base / f'node-manager-{manager}-{dependency_count}'
        shutil.copytree(root / fixture, project)
        before = source_files(project)
        tasks = invoke(project,'run','list','--json')
        assert tasks['project:test']['argv'] == [manager,'run','test']
        assert tasks['project:build']['build_stage'] and tasks['project:test']['build_stage']
        invoke(project,'build')
        manifest = validate(project / 'dist')
        assert manifest['status'] == 'succeeded' and source_files(project) == before
        assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == (dependency_count + 1 if dependency_count else 2)
        assert next(r for r in manifest['reports'] if r['kind'] == 'coverage')['summary']['covered'] > 0
        artifact = manifest['artifacts'][0]
        assert '-dev.g' in artifact['version']
        with tarfile.open(project / 'dist' / artifact['path']) as package:
            assert json.load(package.extractfile('package/package.json'))['version'] == artifact['version']
            assert 'package/dist/greeting.mjs' in package.getnames()
        record = json.loads((project / 'dist/dependencies/project.json').read_text())
        Draft202012Validator(schema).validate(record)
        assert record['manager']['id'] == manager and record['manager']['version'] == version
        assert len(record['packages']) == dependency_count
        if dependency_count:
            expected = {('is-odd', '3.0.1'), ('is-number', '6.0.0')}
            if manager == 'yarn':
                expected.add(('@colors/colors', '1.6.0'))
            assert {(p['name'], p['version']) for p in record['packages']} == expected
            assert all(p['sourceId'] == 'npm-public' for p in record['packages'])
            assert record['extensions'][f'oyzu.dev/{manager}']['integrity'] == 'lockfile-sha512'
        invoke(project,'inspect','dist')
        invoke(project,'build')
        repeated = validate(project / 'dist')
        assert repeated['planDigest'] == manifest['planDigest']
        assert repeated['artifacts'][0]['digest'] == artifact['digest']
        # A failed native runner still delivers both reports and blocks packaging.
        (project / 'test/failing.test.mjs').write_text("import test from 'node:test'; test('expected failure',()=>{throw new Error('expected')});\n")
        invoke(project,'build',success=False)
        failed = validate(project / 'dist')
        assert not failed['artifacts'] and all(r['status'] == 'collected' for r in failed['reports'])
        assert next(r for r in failed['reports'] if r['kind'] == 'test')['summary']['failed'] == 1
        # Contradictory native manager evidence fails before executing a task.
        (project / 'package-lock.json').write_text('{}')
        invoke(project,'build',success=False)
        conflict = validate(project / 'dist')
        assert not conflict['actions'] and not conflict['artifacts']
        assert 'conflicting' in conflict['diagnostics'][0]['message']
        verified.append(f'EX-019 {manager} ({dependency_count} registry packages): native frozen preparation, version evidence, scripts/reports, repeatable snapshot package, failed tests and conflicting locks')
