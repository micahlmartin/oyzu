"""Real Vite output, directory integrity and Docker consumer acceptance."""
import json
import shutil

from .docker import image_contents


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'vite-container'
    shutil.copytree(root/'examples/builds/materialize-directory/variants/vite', project,
                    ignore=shutil.ignore_patterns('node_modules', 'dist'))
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    for name in ['build', 'test', 'lint', 'format-check']:
        assert tasks[f'frontend:{name}']['build_stage']
    assert tasks['frontend:format']['mutates_source']
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status']=='succeeded' and source_files(project)==before
    directory = next(a for a in manifest['artifacts'] if a['target']=='frontend')
    image = next(a for a in manifest['artifacts'] if a['target']=='image')
    assert directory['kind']=='directory' and directory['name']=='primary'
    assert '-dev.g' in directory['version'] and directory['version'] in directory['path']
    entries = {e['path']:e for e in directory['entries']}
    assert 'index.html' in entries and any(p.startswith('assets/') and p.endswith('.js') for p in entries)
    assert not any(p.startswith(('node_modules/', 'src/', 'test/')) for p in entries)
    _, _, contents = image_contents(project/'dist'/image['path'])
    for path, entry in entries.items():
        if entry['kind']=='file':
            assert contents[f'site/{path}']==(project/'dist'/directory['path']/path).read_bytes()
    assert 'site/dist/index.html' not in contents
    receipt, = json.loads((project/'dist/inputs/image.json').read_text())['inputs']
    assert receipt['digest']==directory['digest']
    assert any(r['kind']=='test' and r['target']=='frontend' and r['summary']['passed']==1 for r in manifest['reports'])
    assert any(r['kind']=='coverage' and r['target']=='frontend' and r['summary']['covered']>0 for r in manifest['reports'])
    for stage in ['build', 'test', 'lint', 'format-check', 'package']:
        assert next(a for a in manifest['actions'] if a['id']==f'frontend:{stage}')['status']=='succeeded'
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest']==manifest['planDigest']
    assert [(a['target'],a['digest']) for a in repeated['artifacts']]==[(a['target'],a['digest']) for a in manifest['artifacts']]
    (project/'dist'/directory['path']/'index.html').write_text('tampered')
    invoke(project, 'inspect', 'dist', success=False)

    package_file = project/'frontend/package.json'
    package = json.loads(package_file.read_text())
    package['scripts']['build'] = 'vite build --outDir public-site'
    package_file.write_text(json.dumps(package))
    (project/'frontend/public-site').mkdir()
    (project/'frontend/public-site/stale.js').write_text('stale output must not survive')
    custom_source = source_files(project)
    invoke(project, 'build')
    custom = validate(project/'dist')
    custom_dir = next(a for a in custom['artifacts'] if a['target']=='frontend')
    assert custom_dir['digest']==directory['digest']
    assert source_files(project)==custom_source
    # Check a producer gate after a successful image: stale output cannot pass.
    test_file = project/'frontend/test/greeting.test.js'
    test_file.write_text(test_file.read_text().replace('Hello, Oyzu!', 'incorrect'))
    failed_source = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and source_files(project)==failed_source
    assert next(a for a in failed['actions'] if a['id']=='frontend:test')['status']=='failed'
    assert any(r['target']=='frontend' and r['kind']=='test' and r['summary']['failed']==1 for r in failed['reports'])
    assert not any(a['id']=='image:build' and a['status']=='succeeded' for a in failed['actions'])
    verified.append('Vite: captured native dependencies, directory snapshot, JUnit/coverage, lint/read-only format, flattened Docker input, repeatability, native outDir, tamper and failed-producer rejection; EX-050 matrix remains pending')
