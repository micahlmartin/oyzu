"""Native public pip acquisition followed by offline Dockerfile consumption.

This is a composition check toward EX-058, not private-credential acceptance.
"""
import json

from .docker import image_contents


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'docker-pip-context'
    project.mkdir()
    (project / 'build.yaml').write_text('image:\n  uses: docker/image\n')
    (project / 'requirements.txt').write_text('six==1.17.0\n')
    dockerfile = '''FROM python:3.13-slim-bookworm
WORKDIR /app
COPY requirements.txt .
RUN --mount=type=bind,from=dependencies,target=/dependencies pip install --no-index --no-cache-dir --no-compile --find-links=/dependencies -r requirements.txt
RUN python -c "import six; assert six.__version__ == '1.17.0'"
CMD ["python", "-c", "import six; print(six.__version__)"]
'''
    (project / 'Dockerfile').write_text(dockerfile)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list')
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
