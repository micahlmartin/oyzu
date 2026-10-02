"""Native pnpm store export, offline Docker consumption and source patches."""
import json
import re
import shutil

from .docker import image_contents

IMAGE = 'oyzu-toolchain/node:pnpm10.11.0-node22'


def dockerfile(patched):
    inputs = 'COPY pnpm-workspace.yaml ./\nCOPY patches/ ./patches/\n' if patched else ''
    return f'''FROM {IMAGE}
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
{inputs}RUN --mount=type=bind,from=dependencies,target=/dependencies \\
    cp -r /dependencies /tmp/pnpm-store \\
    && pnpm --config.manage-package-manager-versions=false install --offline --frozen-lockfile --ignore-scripts --ignore-pnpmfile --config.verify-store-integrity=true --config.side-effects-cache=false --store-dir /tmp/pnpm-store \\
    && rm -rf /tmp/pnpm-store
RUN node -e "if (!require('is-odd')(3)) process.exit(1)"
CMD ["node", "-e", "console.log(require('is-odd')(3))"]
'''


def verify(root, base, invoke, validate, source_files, verified):
    for patched in [False, True]:
        project = base / ('docker-pnpm-context-patched' if patched else 'docker-pnpm-context')
        fixture = 'examples/builds/node-managers/variants/pnpm-patches' if patched else 'tooling/fixtures/pnpm-registry'
        shutil.copytree(root / fixture, project)
        package_path = project / 'package.json'
        package = json.loads(package_path.read_text())
        package['scripts']['preinstall'] = 'node -e "process.exit(91)"'
        package_path.write_text(json.dumps(package))
        (project / 'build.yaml').write_text('image:\n  uses: docker/image\n')
        (project / 'Dockerfile').write_text(dockerfile(patched))
        before = source_files(project)
        invoke(project, 'build')
        manifest = validate(project / 'dist')
        assert manifest['status'] == 'succeeded' and source_files(project) == before
        artifact, = manifest['artifacts']
        assert artifact['kind'] == 'oci-image' and '-dev.g' in artifact['version']
        _, _, files = image_contents(project / 'dist' / artifact['path'])
        for name, version in [('is-odd', '3.0.1'), ('is-number', '6.0.0')]:
            manifests = [body for path, body in files.items() if path.startswith('app/node_modules/.pnpm/') and path.endswith(f'/node_modules/{name}/package.json')]
            assert len(manifests) == 1 and json.loads(manifests[0])['version'] == version
        assert not any(name.startswith(('dependencies/', 'tmp/pnpm-store/')) for name in files)
        if patched:
            source, = [body for path, body in files.items() if path.startswith('app/node_modules/.pnpm/') and path.endswith('/node_modules/is-number/index.js')]
            assert b'native-patch-applied' in source
        dependency = json.loads((project / 'dist/dependencies/image.json').read_text())
        context = dependency['extensions']['oyzu.dev/docker']['dependencyContext']
        assert context['provider'] == 'node/pnpm' and context['runtime']['reference'] == IMAGE
        assert context['snapshot']['manager']['id'] == 'pnpm'
        assert context['snapshot']['manager']['version'] == '10.11.0'
        extension = context['snapshot']['extensions']['oyzu.dev/pnpm']
        assert extension['nodeVersion'].startswith('22.')
        if patched:
            assert extension['sourcePatches'][0]['path'] == 'patches/is-number@6.0.0.patch'
        assert {(p['name'], p['version']) for p in dependency['packages']} == {('is-odd', '3.0.1'), ('is-number', '6.0.0')}
        assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
        plan = json.loads((project / 'dist/plan.json').read_text())
        assert all(a['network'] == 'none' for a in plan['actions'])
        invoke(project, 'inspect', 'dist')
        repeated = invoke(project, 'build')
        assert repeated['planDigest'] == manifest['planDigest']
        assert repeated['artifacts'][0]['digest'] == artifact['digest']
        assert source_files(project) == before
        lock_path = project / 'pnpm-lock.yaml'
        lock_path.write_text(re.sub(r'sha512-[A-Za-z0-9+/]{86}==', 'sha512-' + 'A' * 86 + '==', lock_path.read_text()))
        invoke(project, 'build', success=False)
        failed = validate(project / 'dist')
        assert not failed['actions'] and not failed['artifacts']
        assert 'integrity' in json.dumps(failed['diagnostics']).lower()
        verified.append(f'Docker pnpm context (patches={patched}): native frozen preparation, offline store install/import after temporary store cleanup, exact transitive files and patch evidence, snapshot OCI/JUnit/quality, unchanged sources, repeated identities and rejected lock integrity; private sources and workspaces remain pending')
