"""Native public pip acquisition followed by offline Dockerfile consumption.

This is a composition check toward EX-058, not private-credential acceptance.
"""
import json
import subprocess
import re

from python_context_fixtures import IMAGES, create, lock_command

from .docker import image_contents


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'docker-pip-context'
    project.mkdir()
    (project / 'build.yaml').write_text('image:\n  uses: docker/image\n')
    (project / 'requirements.txt').write_text('six==1.17.0\n')
    dockerfile = '''FROM python:3.13-slim-bookworm
WORKDIR /app
COPY requirements.txt .
RUN --mount=type=bind,from=dependencies,target=/dependencies \\
    pip install --no-index --no-cache-dir --no-compile --find-links=/dependencies -r requirements.txt
RUN python -B -c "import six; assert six.__version__ == '1.17.0'"
CMD ["python", "-c", "import six; print(six.__version__)"]
'''
    (project / 'Dockerfile').write_text(dockerfile)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all('image:' + name in tasks for name in ['build', 'test', 'lint', 'format-check'])
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    artifact, = manifest['artifacts']
    assert artifact['kind'] == 'oci-image' and '-dev.g' in artifact['version']
    _, _, contents = image_contents(project / 'dist' / artifact['path'])
    assert b'__version__ = "1.17.0"' in contents['usr/local/lib/python3.13/site-packages/six.py']
    assert not any(name.startswith('dependencies/') for name in contents)
    dependency = json.loads((project / 'dist/dependencies/image.json').read_text())
    context = dependency['extensions']['oyzu.dev/docker']['dependencyContext']
    assert context['provider'] == 'python/pip'
    assert context['runtime']['reference'] == 'python:3.13-slim-bookworm'
    assert context['snapshot']['manager']['platform']['runtime'].startswith('3.13.')
    assert [(p['name'], p['version'], p['purpose']) for p in dependency['packages']] == [('six', '1.17.0', 'runtime')]
    plan = json.loads((project / 'dist/plan.json').read_text())
    build = next(a for a in plan['actions'] if a['id'] == 'image:build')
    assert build['extensions']['oyzu.dev/executor']['dependency_context'] == context['binding']
    assert all(a['network'] == 'none' for a in plan['actions'])
    assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    (project / 'package.json').write_text('{"name":"another-ecosystem","version":"1.0.0"}\n')
    invoke(project, 'build', success=False)
    ambiguous = validate(project / 'dist')
    assert not ambiguous['actions'] and not ambiguous['artifacts']
    assert 'unambiguous ecosystem' in ambiguous['diagnostics'][0]['message']
    (project / 'build.yaml').write_text('image:\n  uses: docker/image\n  dependencies: python/pip\n')
    invoke(project, 'build')
    assert validate(project / 'dist')['status'] == 'succeeded'
    (project / 'Dockerfile').write_text(dockerfile + 'RUN pip install --no-cache-dir --timeout 1 --retries 0 --index-url https://pypi.org/simple six==1.16.0\n')
    invoke(project, 'build', success=False)
    offline = validate(project / 'dist')
    assert not offline['artifacts']
    assert next(a for a in offline['actions'] if a['id'] == 'image:build')['status'] == 'failed'
    verified.append('Docker dependency context: native pip resolves in the exact Python 3.13 base, installs offline from captured wheels, retains package/runtime evidence and OCI/JUnit/quality results, repeats identities, requires explicit selection for ambiguous ecosystems, and blocks build-time network fetches; private-source and other-manager EX-058 requirements remain pending')
    locked_profiles(base, invoke, validate, source_files, verified)


def locked_profiles(base, invoke, validate, source_files, verified):
    for manager, image in IMAGES.items():
        project = base / f'docker-{manager}-context'
        create(project, manager)
        # Fixture setup obtains real native lock hashes before Oyzu captures the
        # project. It is not a network fallback during an Oyzu build.
        command = lock_command(manager, '/usr/local/bin/python')
        result = subprocess.run(['docker', 'run', '--rm', '--pull=never', '--mount', f'type=bind,source={project},target=/workspace', '--workdir', '/workspace', '--entrypoint', command[0], image, *command[1:]], capture_output=True, text=True, timeout=300)
        assert result.returncode == 0, (result.stdout, result.stderr)
        (project / 'build.yaml').write_text('image:\n  uses: docker/image\n')
        installer = 'uv pip install --python /usr/local/bin/python --no-cache' if manager == 'uv' else 'pip install --no-cache-dir --no-compile'
        (project / 'Dockerfile').write_text(f'''FROM {image}
WORKDIR /app
RUN --mount=type=bind,from=dependencies,target=/dependencies \\
    {installer} --target /app/site --no-deps --require-hashes --no-index --find-links=/dependencies -r /dependencies/requirements.txt
RUN PYTHONPATH=/app/site python -B -S -c "import six; assert six.__version__ == '1.17.0'; assert six.__file__.startswith('/app/site/')"
CMD ["python", "-S", "/app/site/six.py"]
''')
        before = source_files(project)
        invoke(project, 'build')
        manifest = validate(project / 'dist')
        assert manifest['status'] == 'succeeded' and source_files(project) == before
        artifact, = manifest['artifacts']
        assert '-dev.g' in artifact['version']
        _, _, contents = image_contents(project / 'dist' / artifact['path'])
        assert b'__version__ = "1.17.0"' in contents['app/site/six.py']
        dependency = json.loads((project / 'dist/dependencies/image.json').read_text())
        context = dependency['extensions']['oyzu.dev/docker']['dependencyContext']
        assert context['provider'] == 'python/' + manager
        assert context['snapshot']['manager']['id'] == manager
        assert context['snapshot']['manager']['version'] == {'uv':'0.12.21', 'poetry':'2.5.1'}[manager]
        assert [(p['name'], p['version']) for p in dependency['packages']] == [('six', '1.17.0')]
        assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
        repeated = invoke(project, 'build')
        assert repeated['planDigest'] == manifest['planDigest']
        assert repeated['artifacts'][0]['digest'] == artifact['digest']
        lock = project / ('uv.lock' if manager == 'uv' else 'poetry.lock')
        lock.write_text(re.sub(r'sha256:[a-f0-9]{64}', 'sha256:' + '0' * 64, lock.read_text()))
        invoke(project, 'build', success=False)
        rejected = validate(project / 'dist')
        assert not rejected['actions'] and not rejected['artifacts']
        assert 'hash' in json.dumps(rejected['diagnostics']).lower()
        verified.append(f'Docker {manager} dependency context: native locked runtime closure excludes development groups, offline hashed installation/import in a private target, snapshot OCI/JUnit/quality results, repeatability, unchanged source locks and rejected lock hashes; private registries remain pending')
