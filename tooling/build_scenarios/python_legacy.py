"""Legacy native Python distributions through captured metadata and real compilation."""
import json
from pathlib import Path
import shutil
import subprocess
import zipfile


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'python-legacy-native'
    shutil.copytree(root/'examples/builds/python-legacy-native/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all('project:'+name in tasks for name in ['build', 'test', 'lint', 'format-check'])
    plan = invoke(project, 'build', '--plan')
    assert plan==invoke(project, 'build', '--plan')
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert source_files(project)==before
    wheel = next(a for a in manifest['artifacts'] if a['name']=='wheel')
    assert '.dev0+g' in wheel['version'] and '-cp312-cp312-linux_x86_64.whl' in wheel['path']
    assert len(manifest['artifacts'])==2
    with zipfile.ZipFile(project/'dist'/wheel['path']) as archive:
        assert any(n.startswith('native_math.') and n.endswith('.so') for n in archive.namelist())
        assert 'fallback.py' in archive.namelist()
        metadata = next(n for n in archive.namelist() if n.endswith('.dist-info/METADATA'))
        assert 'Version: '+wheel['version'] in archive.read(metadata).decode()
    assert any(r['kind']=='test' and r['summary']['passed']==2 for r in manifest['reports'])
    assert any(r['kind']=='coverage' and r['summary']['covered']>0 for r in manifest['reports'])
    # Install only delivered bytes in a clean interpreter and exercise the C module.
    result = subprocess.run([
        'docker', 'run', '--rm', '--network=none', '--mount',
        f'type=bind,source={project/"dist"/wheel["path"]},target=/artifacts/{Path(wheel["path"]).name},readonly',
        'python:3.12-bookworm', 'sh', '-ec',
        'python -m pip install --no-index --no-deps /artifacts/*.whl && python -I -c "import native_math; assert native_math.add(2, 3)==5"',
    ], capture_output=True, text=True, timeout=60)
    assert result.returncode==0, (result.stdout,result.stderr)
    repeated = invoke(project, 'build')
    assert repeated['planDigest']==manifest['planDigest']
    assert {a['name']:a['digest'] for a in repeated['artifacts']}=={a['name']:a['digest'] for a in manifest['artifacts']}

    test = project/'tests/test_math.py'
    test.write_text(test.read_text().replace('native_math.add(2, 3), 5', 'native_math.add(2, 3), 6'))
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert any(r['kind']=='test' and r['summary']['failed']==1 for r in failed['reports'])
    assert any(r['kind']=='coverage' for r in failed['reports'])

    setup = project/'setup.py'
    setup.write_text('from pathlib import Path\nassert not Path("/broker").exists(), "metadata received registry access"\n'+setup.read_text())
    invoke(project, 'build', '--plan')
    invoke(project, 'build', '--image', 'pip=python:3.12-slim-bookworm', success=False)
    failed = validate(project/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'compiler is unavailable' in json.dumps(failed['diagnostics'])
    verified.append('Legacy setuptools: isolated captured metadata/compiler facts, native C-extension snapshot wheel/sdist, installed artifact tests, Python coverage, repeatability and missing-compiler preflight failure')
