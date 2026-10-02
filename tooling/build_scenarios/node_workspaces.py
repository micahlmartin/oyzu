"""EX-020 package/report production through the actual CLI and isolated executor."""
import json
import shutil
import tarfile
from .node_fixtures import format_sources


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'node-workspace'
    shutil.copytree(root/'examples/builds/node-workspace/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all(f'project:{name}' in tasks for name in ['build','test','lint','format-check'])
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status'] == 'succeeded' and before == source_files(project)
    for stage in ['lint','format-check']:
        assert next(a for a in manifest['actions'] if a['id'] == f'project:{stage}')['status'] == 'succeeded'
    assert len(manifest['artifacts']) == 2
    versions = {a['version'] for a in manifest['artifacts']}
    assert len(versions) == 1 and '-dev.g' in next(iter(versions))
    for artifact in manifest['artifacts']:
        with tarfile.open(project/'dist'/artifact['path']) as tar:
            package = json.load(tar.extractfile('package/package.json'))
            assert package['version'] == artifact['version']
            if package['name'].endswith('/app'):
                assert package['dependencies']['@oyzu-example/shared'] in versions
    tests = [r for r in manifest['reports'] if r['kind'] == 'test']
    coverage = [r for r in manifest['reports'] if r['kind'] == 'coverage']
    assert len(tests) == len(coverage) == 2
    assert all(t['summary']['passed'] == 1 and t['summary']['failed'] == 0 for t in tests)
    assert all(c['summary']['covered'] > 0 for c in coverage)
    invoke(project, 'inspect', 'dist')
    invoke(project, 'build')
    repeated = validate(project/'dist')
    assert repeated['planDigest'] == manifest['planDigest']
    assert {a['name']:a['digest'] for a in repeated['artifacts']} == {a['name']:a['digest'] for a in manifest['artifacts']}
    (project/'packages/shared/failing.test.mjs').write_text("import {test} from 'node:test'; test('failure',()=>{throw Error('expected')});\n")
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert failed['status'] == 'failed' and not failed['artifacts']
    tests = [r for r in failed['reports'] if r['kind'] == 'test']
    assert len(tests) == 2 and sum(t['summary']['failed'] for t in tests) == 1
    assert next(a for a in failed['actions'] if a['id'] == 'project:package')['status'] == 'blocked'
    (project/'packages/shared/failing.test.mjs').unlink()
    source = project/'packages/shared/index.mjs'
    original = source.read_text()
    source.write_text(original + '\nconst unused = 1;\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id'] == 'project:lint')['status'] == 'failed'
    source.write_text(original + '\nexport const additional=1;\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id'] == 'project:lint')['status'] == 'succeeded'
    assert next(a for a in failed['actions'] if a['id'] == 'project:format-check')['status'] == 'failed'
    source.write_text(original)
    package_path = project/'packages/shared/package.json'
    package = json.loads(package_path.read_text())
    package['scripts'].update({'lint':'node --check index.mjs', 'format:check':'node --check index.mjs'})
    package_path.write_text(json.dumps(package))
    before = source_files(project)
    invoke(project, 'build')
    mixed = validate(project/'dist')
    assert source_files(project) == before and len(mixed['artifacts']) == 2
    formatting = [a for a in mixed['actions'] if a['id'] in ['project:format-check','project:format:check']]
    assert len(formatting) == 1 and formatting[0]['status'] == 'succeeded'
    (project/'packages/app/quality.mjs').write_text('export const value = missing;\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id'] == 'project:lint')['status'] == 'failed'
    (project/'packages/app/quality.mjs').unlink()
    # A public root is a package as well as a workspace coordinator. No Oyzu
    # configuration or root test script is necessary for conventional tests.
    root_manifest = project/'package.json'
    root_package = json.loads(root_manifest.read_text())
    root_package.pop('private', None)
    root_manifest.write_text(json.dumps(root_package))
    (project/'index.mjs').write_text('export const value = 42;\n')
    root_test = project/'root.test.mjs'
    root_test.write_text("import {test} from 'node:test'; import assert from 'node:assert/strict'; import {value} from './index.mjs'; test('root package',()=>assert.equal(value,42));\n")
    format_sources(root, project/'index.mjs', root_test)
    before = source_files(project)
    invoke(project, 'build')
    public = validate(project/'dist')
    assert public['status'] == 'succeeded' and source_files(project) == before
    assert len(public['artifacts']) == 3
    tests = [r for r in public['reports'] if r['kind'] == 'test']
    coverage = [r for r in public['reports'] if r['kind'] == 'coverage']
    assert len(tests) == len(coverage) == 3
    assert all(t['summary']['passed'] == 1 and t['summary']['failed'] == 0 for t in tests)
    assert all(c['summary']['covered'] > 0 for c in coverage)
    for artifact in public['artifacts']:
        with tarfile.open(project/'dist'/artifact['path']) as tar:
            package = json.load(tar.extractfile('package/package.json'))
            assert package['version'] == artifact['version']
            if package['name'] == root_package['name']:
                assert 'package/index.mjs' in tar.getnames()
                assert not any('/.oyzu-build/' in n or '/.oyzu/' in n for n in tar.getnames())
    invoke(project, 'build')
    repeat = validate(project/'dist')
    assert repeat['planDigest'] == public['planDigest']
    assert {a['name']:a['digest'] for a in repeat['artifacts']} == {a['name']:a['digest'] for a in public['artifacts']}
    root_test.write_text("import {test} from 'node:test'; test('root failure',()=>{throw Error('expected')});\n")
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert sum(r['summary']['failed'] for r in failed['reports'] if r['kind'] == 'test') == 1
    verified.append('EX-020 npm workspaces: isolated native link replay, per-member snapshot packages/JUnit/coverage, internal snapshot references, stable artifacts and failed-test package blocking; affected selection remains pending')
