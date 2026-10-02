"""EX-020 package/report production through the actual CLI and isolated executor."""
import json
import shutil
import tarfile


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'node-workspace'
    shutil.copytree(root/'examples/builds/node-workspace/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all(f'project:{name}' in tasks for name in ['build','test','lint','format-check'])
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status'] == 'succeeded' and before == source_files(project)
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
    verified.append('EX-020 npm workspaces: isolated native link replay, per-member snapshot packages/JUnit/coverage, internal snapshot references, stable artifacts and failed-test package blocking; affected selection remains pending')
