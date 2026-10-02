"""Requirements-only application artifacts built by the compiled CLI."""
import json
import shutil
import subprocess
import zipfile
import xml.etree.ElementTree as ET


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'python-pip-app'
    shutil.copytree(root/'examples/builds/python-pip/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all(f'project:{task}' in tasks for task in ['install', 'build', 'test', 'lint', 'format-check', 'format'])
    assert tasks['project:format']['mutates_source'] and not tasks['project:format']['build_stage']
    plan = invoke(project, 'build', '--plan')
    assert plan == invoke(project, 'build', '--plan')
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status']=='succeeded' and source_files(project)==before
    for task in ['build', 'test', 'lint', 'format-check', 'package']:
        assert next(a for a in manifest['actions'] if a['id']==f'project:{task}')['status']=='succeeded'
    artifact, = manifest['artifacts']
    assert artifact['name']=='application' and '-dev.g' in artifact['version']
    assert artifact['path'].endswith('.pyz')
    with zipfile.ZipFile(project/'dist'/artifact['path']) as archive:
        metadata = json.loads(archive.read('oyzu-application.json'))
        assert metadata['version']==artifact['version']
        assert metadata['sourceFiles']==['app.py']
        assert metadata['runtimePackages']==['packaging/24.2']
        assert not any(n.startswith(('tests/', 'pytest/', '_pytest/', 'setuptools/')) for n in archive.namelist())
        assert 'pyproject.toml' not in archive.namelist()
    test = next(r for r in manifest['reports'] if r['kind']=='test')
    assert test['summary']['passed']==2
    coverage = next(r for r in manifest['reports'] if r['kind']=='coverage')
    assert coverage['summary']['covered']>0
    classes = ET.parse(project/'dist'/coverage['path']).findall('.//class')
    assert [c.attrib['filename'].split('/')[-1] for c in classes]==['app.py']
    delivered = project/'dist'/artifact['path']
    result = subprocess.run([
        'docker', 'run', '--rm', '--network=none', '--read-only',
        '--mount', f'type=bind,source={delivered},target=/app.pyz,readonly',
        '--entrypoint', 'python', 'python:3.12-slim-bookworm', '-I', '-S', '/app.pyz',
    ], capture_output=True, text=True, timeout=60)
    assert result.returncode==0 and "'status': 'ok'" in result.stdout, (result.stdout,result.stderr)
    repeated = invoke(project, 'build')
    assert repeated['planDigest']==manifest['planDigest']
    assert repeated['artifacts'][0]['digest']==artifact['digest']
    invoke(project, 'inspect', 'dist')

    test_file = project/'tests/test_app.py'
    original_test = test_file.read_text()
    test_file.write_text(original_test.replace('{"status": "ok"}', '{"status": "failed"}'))
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert any(r['kind']=='test' and r['summary']['failed']==1 for r in failed['reports'])
    assert any(r['kind']=='coverage' and r['summary']['covered']>0 for r in failed['reports'])
    test_file.write_text(original_test)

    # Lint and formatting are checks: neither silently edits the checkout.
    app = project/'app.py'
    original_app = app.read_text()
    app.write_text('import json\n'+original_app)
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and source_files(project)==before
    assert next(a for a in failed['actions'] if a['id']=='project:lint')['status']=='failed'
    app.write_text(original_app.replace('return {"status": "ok"}', 'return { "status":"ok" }'))
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and source_files(project)==before
    assert next(a for a in failed['actions'] if a['id']=='project:format-check')['status']=='failed'
    app.write_text(original_app)

    # A successful test cannot authorize a modified application archive.
    (project/'oyzu.toml').write_text('[tasks."project:post_test"]\nargv=["sh","-ec","printf corrupt >> .oyzu-build/application.pyz"]\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:package')['status']=='failed'
    assert any(r['kind']=='test' and r['summary']['passed']==2 for r in failed['reports'])
    (project/'oyzu.toml').unlink()

    # The fixture's native hash policy is enforced by pip before build actions.
    requirements = project/'requirements.txt'
    requirements.write_text('--require-hashes\npackaging==24.2\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'hash' in json.dumps(failed['diagnostics']).lower()
    verified.append('Python requirements application: captured hashed runtime dependencies, repeatable runnable snapshot archive, artifact-source JUnit/coverage, failed-test retention and post-test integrity enforcement')
