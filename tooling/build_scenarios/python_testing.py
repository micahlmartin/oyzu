"""Native Python collection must reach artifact tests beyond directory conventions."""
import shutil


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'python-native-collection'
    shutil.copytree(root/'examples/builds/python-api/project', project)
    original = project/'tests/test_api.py'
    root_test = project/'test_api.py'
    original.rename(root_test)
    (project/'tests').rmdir()
    for layout in ['root', 'configured']:
        if layout == 'configured':
            (project/'checks').mkdir()
            root_test.rename(project/'checks/spec_api.py')
            (project/'pytest.ini').write_text('[pytest]\ntestpaths=checks\npython_files=spec_*.py\n')
            # This is valid lint/format input but must not be imported by pytest.
            root_test.write_text('raise RuntimeError("native testpaths ignored")\n')
        tasks = invoke(project,'run','list','--json')
        assert tasks['api:test']['availability'] is None and tasks['api:test']['build_stage']
        before = source_files(project)
        invoke(project,'build')
        manifest = validate(project/'dist')
        assert manifest['status'] == 'succeeded' and source_files(project) == before
        assert {a['name'] for a in manifest['artifacts']} == {'wheel','sdist','application'}
        assert all('.dev0+g' in a['version'] for a in manifest['artifacts'])
        tests = next(r for r in manifest['reports'] if r['kind']=='test')
        assert tests['summary']['passed'] == 2
        coverage = next(r for r in manifest['reports'] if r['kind']=='coverage')
        assert coverage['summary']['covered'] > 0
        selected = manifest['targets'][0]['extensions']['oyzu.dev/discovery']['test-framework']
        assert selected['selected'] == 'pytest'
    root_test.unlink()
    (project/'pytest.ini').unlink()
    (project/'checks/spec_api.py').unlink()
    (project/'checks').rmdir()
    before = source_files(project)
    invoke(project,'build',success=False)
    empty = validate(project/'dist')
    assert source_files(project) == before and not empty['artifacts']
    action = next(a for a in empty['actions'] if a['id']=='api:test')
    assert action['status'] == 'failed' and action['exitCode'] == 5
    tests = next(r for r in empty['reports'] if r['kind']=='test')
    assert tests['status'] == 'collected' and tests['summary']['total'] == 0
    assert next(a for a in empty['actions'] if a['id']=='api:package')['status'] == 'blocked'
    verified.append('Python native collection: root/configured tests, snapshot wheel/sdist with JUnit and coverage, and retained empty-suite evidence blocking artifacts')
