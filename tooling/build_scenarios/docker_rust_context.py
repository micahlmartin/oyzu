"""Standard Cargo consumes a captured registry without reporting/packaging tools."""
import json
import shutil

from .docker import image_contents

IMAGE = 'rust:1.94.0-slim-bookworm'
DOCKERFILE = f'''FROM {IMAGE} AS build
WORKDIR /src
COPY . .
RUN --mount=type=bind,from=dependencies,target=/dependencies \\
    CARGO_HOME=/tmp/cargo-home CARGO_TARGET_DIR=/tmp/cargo-target CARGO_INCREMENTAL=0 cargo \\
    --config 'source.crates-io.replace-with="oyzu-captured"' \\
    --config 'source.oyzu-captured.local-registry="/dependencies"' \\
    build --release --locked --offline \\
    && cp /tmp/cargo-target/release/oyzu-registry-example /app
RUN test "$(/app)" = "Answer: 42"
FROM debian:bookworm-slim
COPY --from=build /app /app
ENTRYPOINT ["/app"]
'''


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'docker-rust-context'
    shutil.copytree(root / 'examples/builds/rust-app/variants/registry', project)
    (project / 'build.yaml').write_text('image:\n  uses: docker/image\n')
    (project / 'Dockerfile').write_text(DOCKERFILE)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all('image:' + name in tasks for name in ['build', 'test', 'lint', 'format-check'])
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    artifact, = manifest['artifacts']
    assert artifact['kind'] == 'oci-image' and '-dev.g' in artifact['version']
    _, config, files = image_contents(project / 'dist' / artifact['path'])
    assert files['app'].startswith(b'\x7fELF') and 'etc/debian_version' in files
    assert not any(name.startswith(('dependencies/', 'tmp/cargo-home/', 'tmp/cargo-target/', 'src/')) for name in files)
    assert config['config']['Entrypoint'] == ['/app']
    dependency = json.loads((project / 'dist/dependencies/image.json').read_text())
    context = dependency['extensions']['oyzu.dev/docker']['dependencyContext']
    assert context['provider'] == 'rust/cargo' and context['runtime']['reference'] == IMAGE
    assert context['snapshot']['manager']['id'] == 'cargo'
    assert context['snapshot']['manager']['version'].startswith('1.94.')
    extension = context['snapshot']['extensions']
    assert set(extension['oyzu.dev/cargo-tools']) == {'rustc'}
    assert set(extension['oyzu.dev/cargo-workspace']) == {'original'}
    metadata = extension['oyzu.dev/cargo-workspace']['original']
    local = [p for p in metadata['packages'] if p['id'] in metadata['workspace_members']]
    assert [(p['name'], p['version']) for p in local] == [('oyzu-registry-example', '0.1.0')]
    assert [(p['name'], p['version']) for p in dependency['packages']] == [('itoa', '1.0.15')]
    assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
    plan = json.loads((project / 'dist/plan.json').read_text())
    assert all(a['network'] == 'none' for a in plan['actions'])
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    assert source_files(project) == before
    lock = project / 'Cargo.lock'
    lock.write_text(lock.read_text().replace('4a5f13b858c8d314ee3e8f639011f7ccefe71f97f96e50151fb991f267928e2c', '0' * 64))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'checksum' in json.dumps(failed['diagnostics']).lower()
    verified.append('Docker Cargo context: standard Rust consumer without nextest/llvm-cov, unchanged native versions/lock, verified registry capture, offline compilation/run, binary-only application layer, snapshot OCI/JUnit/quality, repeated identities and rejected checksum; source coverage and final-image startup remain separate obligations')
