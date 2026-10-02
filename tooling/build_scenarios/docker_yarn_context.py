"""Native Yarn mirror consumption through the compiled CLI's Docker builder."""
import json
import re
import shutil

from .docker import image_contents

IMAGE = 'oyzu-toolchain/node:yarn1.22.22-node22'
DOCKERFILE = f'''FROM {IMAGE}
WORKDIR /app
COPY package.json yarn.lock ./
RUN --mount=type=bind,from=dependencies,target=/dependencies \\
    printf 'yarn-offline-mirror "/dependencies"\\ndisable-self-update-check true\\n' >/tmp/oyzu.yarnrc \\
    && yarn install --offline --non-interactive --frozen-lockfile --ignore-scripts --use-yarnrc /tmp/oyzu.yarnrc --cache-folder /tmp/yarn-cache \\
    && yarn cache clean --offline --non-interactive --cache-folder /tmp/yarn-cache \\
    && rm -rf /tmp/yarn-cache /tmp/oyzu.yarnrc
RUN node -e "if (!require('is-odd')(3) || !require('@colors/colors')) process.exit(1)"
CMD ["node", "-e", "console.log(require('is-odd')(3))"]
'''


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'docker-yarn-context'
    shutil.copytree(root / 'tooling/fixtures/yarn-registry', project)
    package_path = project / 'package.json'
    package = json.loads(package_path.read_text())
    package['scripts']['preinstall'] = 'node -e "process.exit(91)"'
    package_path.write_text(json.dumps(package))
    (project / 'build.yaml').write_text('image:\n  uses: docker/image\n')
    (project / 'Dockerfile').write_text(DOCKERFILE)
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    artifact, = manifest['artifacts']
    assert artifact['kind'] == 'oci-image' and '-dev.g' in artifact['version']
    _, _, files = image_contents(project / 'dist' / artifact['path'])
    expected = {('is-odd', '3.0.1'), ('is-number', '6.0.0'), ('@colors/colors', '1.6.0')}
    for name, version in expected:
        assert json.loads(files[f'app/node_modules/{name}/package.json'])['version'] == version
    assert not any(name.startswith(('dependencies/', 'tmp/yarn-cache/', 'tmp/oyzu.yarnrc')) for name in files)
    dependency = json.loads((project / 'dist/dependencies/image.json').read_text())
    context = dependency['extensions']['oyzu.dev/docker']['dependencyContext']
    assert context['provider'] == 'node/yarn' and context['runtime']['reference'] == IMAGE
    assert context['snapshot']['manager']['id'] == 'yarn'
    assert context['snapshot']['manager']['version'] == '1.22.22'
    assert context['snapshot']['extensions']['oyzu.dev/yarn']['nodeVersion'].startswith('22.')
    assert {(p['name'], p['version']) for p in dependency['packages']} == expected
    assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
    plan = json.loads((project / 'dist/plan.json').read_text())
    assert all(a['network'] == 'none' for a in plan['actions'])
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    assert source_files(project) == before
    lock_path = project / 'yarn.lock'
    lock_path.write_text(re.sub(r'sha512-[A-Za-z0-9+/]{86}==', 'sha512-' + 'A' * 86 + '==', lock_path.read_text()))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'integrity' in json.dumps(failed['diagnostics']).lower()
    verified.append('Docker Yarn Classic context: native validated locked mirror with scoped/transitive packages, offline install/import, lifecycle suppression, exact runtime/package evidence, snapshot OCI/JUnit/quality, unchanged sources, repeated identities and rejected integrity; private registries and modern Yarn remain pending')
