"""Native workspace snapshots and required stages through the compiled CLI."""
import json
import shutil
import tarfile


def verify_workspace(manager, variant, root, base, invoke, validate, source_files, verified):
    project = base/f'{manager}-workspace'
    shutil.copytree(root/'examples/builds/node-workspace/variants'/variant,project)
    before = source_files(project)
    tasks = invoke(project,'run','list','--json')
    assert all(f'project:{stage}' in tasks for stage in ['build','test','lint','format-check'])
    invoke(project,'build')
    manifest = validate(project/'dist')
    assert manifest['status']=='succeeded' and source_files(project)==before
    for stage in ['build','test','lint','format-check','package']:
        assert next(a for a in manifest['actions'] if a['id']==f'project:{stage}')['status']=='succeeded'
    assert len(manifest['artifacts'])==2
    versions={a['version'] for a in manifest['artifacts']}
    assert len(versions)==1 and '-dev.g' in next(iter(versions))
    for artifact in manifest['artifacts']:
        with tarfile.open(project/'dist'/artifact['path']) as archive:
            package=json.load(archive.extractfile('package/package.json'))
            assert package['version']==artifact['version']
            if package['name'].endswith('/app'):
                assert package['dependencies']['@oyzu-example/shared'] in versions
    tests=[r for r in manifest['reports'] if r['kind']=='test']
    coverage=[r for r in manifest['reports'] if r['kind']=='coverage']
    assert len(tests)==len(coverage)==2
    assert all(r['summary']['passed']==1 for r in tests)
    assert all(r['summary']['covered']>0 for r in coverage)
    invoke(project,'inspect','dist')
    (project/'packages/shared/failure.test.mjs').write_text("import test from 'node:test'; test('failure',()=>{throw Error('expected')});\n")
    invoke(project,'build',success=False)
    failed=validate(project/'dist')
    assert failed['status']=='failed' and not failed['artifacts']
    assert sum(r['summary']['failed'] for r in failed['reports'] if r['kind']=='test')==1
    (project/'packages/shared/failure.test.mjs').unlink()
    source=project/'packages/shared/index.mjs'
    source.write_text(source.read_text()+'\nconst unused = 1;\n')
    invoke(project,'build',success=False)
    failed=validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:lint')['status']=='failed'
    verified.append(f'EX-020 {manager}: offline workspace snapshots, projected local dependencies, native tests/coverage, quality, source preservation, inspected bundles and failure gates')
