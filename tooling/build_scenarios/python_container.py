"""Authored Python application container variant through the compiled CLI."""
import json
import shutil

from .docker import image_contents


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'python-container'
    fixture = root / 'examples/builds/python-api'
    shutil.copytree(fixture / 'project', project)
    shutil.copyfile(fixture / 'variants/container.build.yaml', project / 'build.yaml')
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all(f'api:{stage}' in tasks for stage in ['build', 'test', 'lint', 'format-check'])
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    assert len(manifest['artifacts']) == 4
    application, = [a for a in manifest['artifacts'] if a['name'] == 'application']
    image, = [a for a in manifest['artifacts'] if a['kind'] == 'oci-image']
    assert image['target'] == 'api-container' and '-dev.g' in image['version']
    digest, config, contents = image_contents(project / 'dist' / image['path'])
    assert digest == image['ociDigest']
    assert contents['app/application.pyz'] == (project / 'dist' / application['path']).read_bytes()
    assert config['config']['User'] == '65532:65532'
    assert config['config']['Entrypoint'] == ['python', '/app/application.pyz']
    assert config['config']['WorkingDir'] == '/app'
    plan = json.loads((project / 'dist/plan.json').read_text())
    owner_config = next(t for t in plan['targets'] if t['id'] == 'api')['extensions']['oyzu.dev/configuration']
    derived_config = next(t for t in plan['targets'] if t['id'] == 'api-container')['extensions']['oyzu.dev/configuration']
    assert derived_config == owner_config
    derived_actions = [a for a in plan['actions'] if a['target'] == 'api-container']
    assert all(a['extensions']['oyzu.dev/configuration-digest'] == owner_config['digest'] for a in derived_actions)
    assert plan['policy']['mode'] == 'standalone'
    assert next(a for a in plan['actions'] if a['id'] == 'api-container:build')['dependsOn'] == ['api:package']
    assert [a['id'] for a in plan['actions']].count('api:build') == 1
    receipt, = json.loads((project / 'dist/inputs/api-container.json').read_text())['inputs']
    assert receipt['digest'] == application['digest']
    assert any(r['target'] == 'api' and r['kind'] == 'coverage' and r['summary']['covered'] > 0 for r in manifest['reports'])
    assert any(r['target'] == 'api-container' and r['kind'] == 'test' and r['summary']['passed'] > 0 for r in manifest['reports'])
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert {a['id']: a['digest'] for a in repeated['artifacts']} == {a['id']: a['digest'] for a in manifest['artifacts']}
    shutil.copyfile(fixture / 'variants/container-overrides.build.yaml', project / 'build.yaml')
    customized_source = source_files(project)
    invoke(project, 'build')
    customized = validate(project / 'dist')
    assert customized['status'] == 'succeeded' and source_files(project) == customized_source
    custom_app, = [a for a in customized['artifacts'] if a['name'] == 'application']
    custom_image, = [a for a in customized['artifacts'] if a['kind'] == 'oci-image']
    _, custom_config, custom_contents = image_contents(project / 'dist' / custom_image['path'])
    assert custom_config['config']['User'] == '1000:1001'
    assert custom_config['config']['WorkingDir'] == '/srv/api'
    assert custom_config['config']['Entrypoint'] == ['python', '/app/application.pyz']
    assert custom_contents['app/application.pyz'] == (project / 'dist' / custom_app['path']).read_bytes()
    captured = json.loads((project / 'dist/dependencies/api-container.json').read_text())
    assert captured['extensions']['oyzu.dev/container']['runtime']['reference'] == 'oyzu-fixture/python-runtime:3.12'
    customized_repeat = invoke(project, 'build')
    assert customized_repeat['planDigest'] == customized['planDigest']
    assert {a['id']: a['digest'] for a in customized_repeat['artifacts']} == {a['id']: a['digest'] for a in customized['artifacts']}
    (project / 'oyzu.toml').write_text('[tasks."api:pre_test"]\nargv=["sh", "-ec", "exit 27"]\n')
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id'] == 'api-container:build')['status'] == 'blocked'
    (project / 'oyzu.toml').unlink()
    (project / 'build.yaml').write_text('api:\n  uses: python/app\n  container: {base: alpine:3.22}\n')
    invoke(project, 'build', success=False)
    incompatible = validate(project / 'dist')
    assert not incompatible['actions'] and not incompatible['artifacts']
    assert 'provisioned container runtime does not satisfy' in incompatible['diagnostics'][0]['message']
    verified.append('EX-012 container variants: one native application build, exact tested capsule in a versioned OCI image, captured compatible default/custom runtime, numeric user/workdir/argv overrides, application JUnit/coverage plus image assertions, stable inputs/outputs, failed-test gate and incompatible runtime rejection; runtime smoke execution and matrix profiles remain pending')
