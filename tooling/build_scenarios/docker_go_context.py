"""A standard Go image consumes captured modules without a packaging adapter."""
import json
import shutil

from .docker import image_contents

IMAGE = 'golang:1.24-bookworm'
DOCKERFILE = f'''FROM {IMAGE} AS build
WORKDIR /src
COPY . .
RUN --mount=type=bind,from=dependencies,target=/dependencies \\
    GOMODCACHE=/dependencies GOPROXY=off GOSUMDB=off GOTOOLCHAIN=local CGO_ENABLED=0 go build -mod=readonly -trimpath -buildvcs=false -o /app .
RUN test "$(/app)" = "00000000-0000-0000-0000-000000000000"
FROM scratch
COPY --from=build /app /app
ENTRYPOINT ["/app"]
'''


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'docker-go-context'
    shutil.copytree(root / 'tooling/fixtures/go-registry', project)
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
    assert set(files) == {'app'} and files['app'].startswith(b'\x7fELF')
    assert config['config']['Entrypoint'] == ['/app']
    dependency = json.loads((project / 'dist/dependencies/image.json').read_text())
    context = dependency['extensions']['oyzu.dev/docker']['dependencyContext']
    assert context['provider'] == 'go/modules' and context['runtime']['reference'] == IMAGE
    assert context['snapshot']['manager']['id'] == 'go'
    assert context['snapshot']['manager']['version'].startswith('go1.24.')
    assert context['snapshot']['extensions']['oyzu.dev/go-module-artifacts'] == []
    assert [(p['name'], p['version']) for p in dependency['packages']] == [('github.com/google/uuid', 'v1.6.0')]
    assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
    plan = json.loads((project / 'dist/plan.json').read_text())
    assert all(a['network'] == 'none' for a in plan['actions'])
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    assert source_files(project) == before
    checksum_path = project / 'go.sum'
    checksum_path.write_text(checksum_path.read_text().replace('NIvaJDMOsjHA8n1jAhLSgzrAzy1Hgr+hNrb57e+94F0=', 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA='))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'checksum mismatch' in json.dumps(failed['diagnostics']).lower()
    verified.append('Docker Go context: standard provisioned Go consumer image without the modulezip helper, native go.sum-verified acquisition, read-only module-cache compilation/run, exact scratch binary output, snapshot OCI/JUnit/quality, unchanged sources, repeated identities and rejected checksum; no source coverage or final-image startup claimed')
