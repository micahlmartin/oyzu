"""Native quality selection through a complete captured Python package build."""
import json
import shutil


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'python-black-flake8'
    shutil.copytree(root/'examples/builds/python-api/project', project)
    metadata = project/'pyproject.toml'
    metadata.write_text(metadata.read_text()+'\n[tool.black]\nextend-exclude="/generated/"\n')
    (project/'.flake8').write_text('[flake8]\nextend-exclude=generated\nmax-line-length=88\nexit-zero=true\n')
    (project/'generated').mkdir()
    (project/'generated/broken.py').write_text('invalid Python !!!\n')
    tasks = invoke(project, 'run', 'list', '--json')
    assert tasks['api:lint']['argv']==['flake8', '.']
    assert tasks['api:format-check']['argv']==['black', '--check', '.']
    assert tasks['api:format']['mutates_source']
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert source_files(project)==before
    assert {a['name'] for a in manifest['artifacts']}=={'wheel', 'sdist'}
    assert all('.dev0+g' in a['version'] for a in manifest['artifacts'])
    for task in ['build', 'test', 'lint', 'format-check', 'package']:
        assert next(a for a in manifest['actions'] if a['id']==f'api:{task}')['status']=='succeeded'
    assert any(r['kind']=='test' and r['summary']['passed']>0 for r in manifest['reports'])
    assert any(r['kind']=='coverage' and r['summary']['covered']>0 for r in manifest['reports'])
    dependencies = json.loads((project/'dist/dependencies/api.json').read_text())
    names = {p['name'] for p in dependencies['packages']}
    assert {'black', 'flake8'} <= names and 'ruff' not in names
    code = project/'api/__init__.py'
    original = code.read_text()
    code.write_text('import socket\n'+original)
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and source_files(project)==before
    assert next(a for a in failed['actions'] if a['id']=='api:lint')['status']=='failed'
    # Quote choice is valid Python/Flake8 and fails only the native formatter.
    code.write_text(original.replace('"/health"', "'/health'"))
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and source_files(project)==before
    assert next(a for a in failed['actions'] if a['id']=='api:format-check')['status']=='failed'
    verified.append('Python quality: implicit native Black/Flake8 selection, captured tools, preserved exclusions, snapshot wheel/sdist and non-mutating failure gates')
