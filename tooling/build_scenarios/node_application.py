"""Real Vite output, directory integrity and Docker consumer acceptance."""
import json
import shutil

from .docker import image_contents
from .node_fixtures import format_sources


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

    package['scripts']['build'] = 'vite build'
    package_file.write_text(json.dumps(package))
    shutil.copyfile(root/'examples/builds/materialize-directory/variants/vite-config/vite.config.mjs',
                    project/'frontend/vite.config.mjs')
    # A configured native output and extra metadata artifact must still select
    # the primary directory automatically for Docker materialization.
    shutil.rmtree(project/'frontend/public-site')
    configured_source = source_files(project)
    invoke(project, 'build')
    configured = validate(project/'dist')
    assert source_files(project)==configured_source
    configured_dir = next(a for a in configured['artifacts'] if a['target']=='frontend' and a['name']=='primary')
    metadata = next(a for a in configured['artifacts'] if a['target']=='frontend' and a['name']=='build-metadata')
    native = json.loads((project/'dist'/metadata['path']).read_text())
    assert native['output']=='web/production' and native['version']==configured_dir['version']
    assert native['toolVersion'].startswith('8.') and native['mode']=='production'
    configured_image = next(a for a in configured['artifacts'] if a['target']=='image')
    _, _, configured_contents = image_contents(project/'dist'/configured_image['path'])
    assert configured_contents['site/native-mode.txt']==b'production'
    for entry in configured_dir['entries']:
        if entry['kind']=='file':
            assert configured_contents[f"site/{entry['path']}"]==(project/'dist'/configured_dir['path']/entry['path']).read_bytes()
    invoke(project, 'inspect', 'dist')
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
    verified.append('Vite: captured dependencies, native config/plugin outputs and metadata, directory snapshot, JUnit/coverage, quality gates, flattened Docker input, repeatability, native outDir, tamper and failed-producer rejection; this group covers single-platform assembly; matrix qualification belongs to the Docker platform-execution group')
    verify_conventional(root, base, invoke, validate, source_files, verified)


def verify_conventional(root, base, invoke, validate, source_files, verified):
    project = base/'custom-script-container'
    shutil.copytree(root/'examples/builds/materialize-directory/project', project,
                    ignore=shutil.ignore_patterns('node_modules', 'dist'))
    format_sources(root, project/'frontend')
    before = source_files(project)
    # Exercise the authored producer with the image matrix still present, but
    # not selected. The Docker platform-execution group checks the authored matrix.
    invoke(project, 'build', 'frontend')
    producer = validate(project/'dist')
    assert producer['status'] == 'succeeded' and source_files(project) == before
    directory, = producer['artifacts']
    assert directory['kind'] == 'directory' and directory['name'] == 'primary'
    assert directory['version'] in directory['path'] and '-dev.g' in directory['version']
    assert {e['path'] for e in directory['entries']} == {'index.html'}
    assert b'Hello, Oyzu!' in (project/'dist'/directory['path']/'index.html').read_bytes()
    assert all(r['status'] == 'collected' for r in producer['reports'])
    assert {r['kind'] for r in producer['reports']} == {'test', 'coverage'}

    # A native-platform variant verifies actual materialization and OCI bytes.
    # This is not evidence that the original two-platform scenario passed.
    build_file = project/'build.yaml'
    build_file.write_text(build_file.read_text().replace('  matrix:\n    platform: [linux/amd64, linux/arm64]\n', ''))
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    directory = next(a for a in manifest['artifacts'] if a['target'] == 'frontend')
    image = next(a for a in manifest['artifacts'] if a['target'] == 'image')
    _, _, contents = image_contents(project/'dist'/image['path'])
    assert contents['site/index.html'] == (project/'dist'/directory['path']/'index.html').read_bytes()
    assert 'site/dist/index.html' not in contents
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert [(a['target'], a['digest']) for a in repeated['artifacts']] == [(a['target'], a['digest']) for a in manifest['artifacts']]
    # A successful custom build that omits dist cannot reuse earlier artifacts.
    script = project/'frontend/build.mjs'
    script.write_text('console.log("build completed without output");\n')
    format_sources(root, script)
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert failed['status'] == 'failed' and failed['artifacts'] == []
    assert source_files(project) == before
    assert next(a for a in failed['actions'] if a['id'] == 'frontend:package')['status'] == 'failed'
    assert not any(a['id'] == 'image:build' and a['status'] == 'succeeded' for a in failed['actions'])
    verified.append('EX-050 custom producer: explicit app default dist, snapshot directory, JUnit/coverage, native-platform Docker materialization, repeatability and missing-output failure; matrix qualification belongs to the Docker platform-execution group')
