"""EX-032: execute both authored runtimes and verify separate native evidence."""
import json
import shutil

from .node_fixtures import format_sources


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'node-runtime-matrix'
    shutil.copytree(root/'examples/builds/compatibility-matrix/project', project)
    format_sources(root, project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    for stage in ['build', 'test', 'lint', 'format-check']:
        assert tasks[f'app:{stage}']['build_stage']
    invoke(project, 'build', 'app')
    manifest = validate(project/'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    targets = {t['variant']['node']: t['id'] for t in manifest['targets']}
    assert set(targets) == {'22.14.0', '24.14.1'}
    assert len(manifest['artifacts']) == 2
    assert len({a['path'] for a in manifest['artifacts']}) == 2
    for version, target in targets.items():
        artifact, = [a for a in manifest['artifacts'] if a['target'] == target]
        assert artifact['variant'] == {'node': version}
        assert artifact['kind'] == 'directory' and '-dev.g' in artifact['version']
        assert artifact['version'] in artifact['path']
        assert (project/'dist'/artifact['path']/'greeting.mjs').is_file()
        dependency = json.loads((project/'dist/dependencies'/f'{target}.json').read_text())
        assert dependency['extensions']['oyzu.dev/npm']['nodeVersion'] == version
        reports = [r for r in manifest['reports'] if r['target'] == target]
        assert {r['kind'] for r in reports} == {'test', 'coverage'}
        assert all(r['status'] == 'collected' for r in reports)
        assert next(r for r in reports if r['kind'] == 'test')['summary']['passed'] > 0
        assert next(r for r in reports if r['kind'] == 'coverage')['summary']['covered'] > 0
        for stage in ['build', 'test', 'lint', 'format-check', 'package']:
            assert next(a for a in manifest['actions'] if a['id'] == f'{target}:{stage}')['status'] == 'succeeded'
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build', 'app')
    assert repeated['planDigest'] == manifest['planDigest']
    assert [(a['target'], a['digest']) for a in repeated['artifacts']] == [(a['target'], a['digest']) for a in manifest['artifacts']]

    # A manager-level override cannot make a mismatched runtime pass admission.
    invoke(project, 'build', '--image', 'npm=oyzu-toolchain/node:npm11.11.0-node24.14.1', success=False)
    failed = validate(project/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'does not match provisioned Node' in failed['diagnostics'][0]['message']

    package_path = project/'package.json'
    package = json.loads(package_path.read_text())
    package['engines']['node'] = '>=24'
    package_path.write_text(json.dumps(package))
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'EBADENGINE' in failed['diagnostics'][0]['message']

    # A runtime-specific test failure retains both variants' test evidence but
    # cannot produce the failing variant's distributable or overwrite its peer.
    package['engines']['node'] = '>=22'
    package_path.write_text(json.dumps(package))
    (project/'oyzu.toml').write_text('[tasks.test]\nargv = ["node", "--test"]\n'
        '[tasks.pre_test]\nargv = ["node", "-e", "console.log(process.versions.node)"]\n'
        '[tasks.post_test]\nargv = ["node", "-e", "require(\'node:fs\').accessSync(process.env.OYZU_TEST_REPORT)"]\n')
    (project/'runtime.test.mjs').write_text("import { test } from 'node:test';\nimport assert from 'node:assert/strict';\ntest('runtime-specific gate', () => assert.equal(process.versions.node, '22.14.0'));\n")
    format_sources(root, project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert failed['status'] == 'failed'
    tests = {r['target']: r for r in failed['reports'] if r['kind'] == 'test'}
    assert tests[targets['22.14.0']]['summary']['failed'] == 0
    assert tests[targets['24.14.1']]['summary']['failed'] > 0
    assert all(a['target'] != targets['24.14.1'] for a in failed['artifacts'])
    assert any(a['target'] == targets['22.14.0'] for a in failed['artifacts'])
    assert next(a for a in failed['actions'] if a['id'] == f"{targets['22.14.0']}:post_root-test")['status'] == 'succeeded'
    invoke(project, 'inspect', 'dist')
    verified.append('EX-032: Node 22.14.0/24.14.1 native preflight, isolated build/test/quality actions, versioned directories, JUnit/coverage, repeatability, runtime mismatch and engine rejection')
