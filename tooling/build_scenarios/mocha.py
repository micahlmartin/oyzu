"""Real captured Mocha tests, native c8 coverage and versioned package output."""
import json
import shutil
import tarfile


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'node-mocha'
    shutil.copytree(root/'tooling/fixtures/mocha', project, ignore=shutil.ignore_patterns('node_modules'))
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert tasks['project:test']['argv']==['npm','run','test']
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status']=='succeeded' and source_files(project)==before
    assert manifest['targets'][0]['extensions']['oyzu.dev/discovery']['test-framework']['selected']=='mocha'
    reports = manifest['reports']
    tests = next(r for r in reports if r['kind']=='test')
    assert tests['summary']['passed']==1 and tests['summary']['skipped']==1
    coverage = next(r for r in reports if r['kind']=='coverage')
    assert coverage['summary']['covered']>0
    assert 'src/greeting.js' in (project/'dist'/coverage['path']).read_text()
    artifact, = manifest['artifacts']
    assert '-dev.g' in artifact['version']
    with tarfile.open(project/'dist'/artifact['path']) as archive:
        assert json.load(archive.extractfile('package/package.json'))['version']==artifact['version']
    invoke(project, 'inspect', 'dist')
    test = project/'test/greeting.test.js'
    original = test.read_text()
    test.write_text(original.replace('"Hello, Oyzu!"', '"wrong"'))
    failed_source = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and source_files(project)==failed_source
    assert next(r for r in failed['reports'] if r['kind']=='test')['summary']['failed']==1
    assert any(r['kind']=='coverage' and r['status']=='collected' for r in failed['reports'])
    test.write_text(original)
    (project/'.c8rc.json').write_text(json.dumps({'check-coverage':True,'branches':100}))
    invoke(project, 'build', success=False)
    threshold = validate(project/'dist')
    assert not threshold['artifacts']
    assert next(r for r in threshold['reports'] if r['kind']=='test')['summary']['failed']==0
    assert any(a['id']=='project:test' and a['status']=='failed' for a in threshold['actions'])
    verified.append('Mocha: captured native framework, c8 toolchain coverage, required JUnit/LCOV, snapshot package, failed assertions and native coverage-threshold gates')

    workspace = base/'mocha-workspace'
    shutil.copytree(root/'tooling/fixtures/mocha-workspace', workspace)
    before = source_files(workspace)
    invoke(workspace, 'build')
    manifest = validate(workspace/'dist')
    assert manifest['status']=='succeeded' and source_files(workspace)==before
    assert len(manifest['artifacts'])==3
    for artifact in manifest['artifacts']:
        assert '-dev.g' in artifact['version']
        with tarfile.open(workspace/'dist'/artifact['path']) as archive:
            assert json.load(archive.extractfile('package/package.json'))['version']==artifact['version']
    for kind in ['test','coverage']:
        reports = [r for r in manifest['reports'] if r['kind']==kind]
        assert len(reports)==3
        assert all(r['summary']['passed' if kind=='test' else 'covered']>0 for r in reports)
        if kind=='test': assert all(r['summary']['passed']==1 for r in reports)
    invoke(workspace, 'inspect', 'dist')
    member = workspace/'packages/implicit/test/member.js'
    member.write_text(member.read_text().replace('value, 11','value, 99'))
    invoke(workspace, 'build', success=False)
    failed = validate(workspace/'dist')
    assert not failed['artifacts']
    tests = [r for r in failed['reports'] if r['kind']=='test']
    assert len(tests)==3 and sum(r['summary']['failed'] for r in tests)==1
    verified.append('Mocha npm workspaces: hoisted native runner, implicit/scripted members, root scope, three versioned packages with per-package JUnit/LCOV and failed-member package blocking')
