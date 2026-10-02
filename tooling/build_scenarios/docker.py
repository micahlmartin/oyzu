"""Real compiled-CLI Dockerfile builds and verified application artifact assembly."""
import hashlib
import io
import json
import shutil
import tarfile


def image_contents(path):
    with tarfile.open(path) as archive:
        files = {m.name.removeprefix('./'): m for m in archive.getmembers() if m.isfile()}

        def blob(descriptor):
            digest = descriptor['digest'].removeprefix('sha256:')
            value = archive.extractfile(files[f'blobs/sha256/{digest}']).read()
            assert hashlib.sha256(value).hexdigest() == digest and len(value) == descriptor['size']
            return value

        root = json.load(archive.extractfile(files['index.json']))['manifests'][0]
        manifest = json.loads(blob(root))
        config = json.loads(blob(manifest['config']))
        contents = {}
        for descriptor in manifest['layers']:
            with tarfile.open(fileobj=io.BytesIO(blob(descriptor)), mode='r:*') as layer:
                for member in layer.getmembers():
                    if member.isfile():
                        contents[member.name.removeprefix('./')] = layer.extractfile(member).read()
        return root['digest'], config, contents


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'docker-offline'
    shutil.copytree(root / 'examples/builds/docker-offline/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert tasks['project:build']['build_stage']
    assert tasks['project:test']['availability']
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    assert any(r['target']=='project' and r['kind']=='test' and r['summary']['passed']==2 for r in manifest['reports'])
    assert manifest['targets'][0]['extensions']['oyzu.dev/coverage-applicability']['status']=='inapplicable'
    tested = next(a for a in manifest['actions'] if a['id']=='project:test')
    assert tested['status']=='succeeded' and 'engine-bounded-oci-validation' in tested['enforced']
    assert 'docker-network-none' not in tested['enforced'], 'engine validation must not claim container execution'
    artifact, = manifest['artifacts']
    assert artifact['kind'] == 'oci-image' and '-dev.g' in artifact['version']
    digest, config, files = image_contents(project / 'dist' / artifact['path'])
    assert digest == artifact['ociDigest'] and files['greeting.txt'] == (project / 'greeting.txt').read_bytes()
    assert config['os'] == 'linux' and config['architecture'] == 'amd64'
    action = next(a for a in manifest['actions'] if a['id'] == 'project:build')
    assert 'buildkit-rootless-worker' in action['enforced'] and 'docker-read-only-root' not in action['enforced']
    invoke(project, 'inspect', 'dist')
    inspected = invoke(project, 'inspect', str(project / 'dist' / artifact['path']))
    assert inspected['ociDigest'] == digest
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['ociDigest'] == digest
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    (project/'oyzu.toml').write_text('[tasks."project:pre_test"]\nargv=["sh","-ec","printf corrupt > /out/project/container/image.tar"]\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:test')['status']=='failed'
    assert any(r['kind']=='test' and r['summary']['failed']==1 and r['summary']['skipped']==1 for r in failed['reports'])
    (project/'oyzu.toml').write_text('[tasks."project:test"]\nargv=["sh","-c","true"]\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and next(a for a in failed['actions'] if a['id']=='project:test')['status']=='failed'
    (project/'oyzu.toml').unlink()
    (project / 'Dockerfile').write_text('FROM scratch\nADD https://example.invalid/unprepared /payload\n')
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['artifacts'] and not failed['actions']
    assert 'add-source' in failed['diagnostics'][0]['message']

    project = base / 'go-container'
    project.mkdir()
    shutil.copytree(root / 'examples/builds/go-app/project', project / 'app')
    (project / 'image').mkdir()
    (project / 'image/Dockerfile').write_text('FROM scratch\nCOPY bin/server /server\nCOPY . /context/\nRUN ["/server"]\nENTRYPOINT ["/server"]\n')
    (project / 'image/.dockerignore').write_text('bin/\nprivate.txt\n')
    (project / 'image/private.txt').write_text('synthetic ignored source marker')
    (project / 'image/bin').mkdir()
    (project / 'image/bin/server').write_text('stale ignored local binary')
    (project / 'build.yaml').write_text('app:\n  uses: go/app\n  path: app\nimage:\n  uses: docker/image\n  path: image\n  materialize:\n    - from: app\n      to: bin/server\n')
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    binary = next(a for a in manifest['artifacts'] if a['target'] == 'app')
    image = next(a for a in manifest['artifacts'] if a['target'] == 'image')
    digest, config, files = image_contents(project / 'dist' / image['path'])
    assert digest == image['ociDigest'] and config['config']['Entrypoint'] == ['/server']
    assert hashlib.sha256(files['server']).hexdigest() == binary['digest'].removeprefix('sha256:')
    assert b'synthetic ignored source marker' not in b''.join(files.values())
    assert b'stale ignored local binary' not in b''.join(files.values())
    assert (project / 'image/bin/server').read_text() == 'stale ignored local binary'
    assert any(r['kind'] == 'test' and r['summary']['passed'] > 0 for r in manifest['reports'])
    assert any(r['target']=='image' and r['kind']=='test' and r['summary']['passed']==2 for r in manifest['reports'])
    assert any(e['kind'] == 'artifact-materialization' and e['id'] == 'image/materialization' for e in manifest['evidence'])
    receipt = json.loads((project/'dist/inputs/image.json').read_text())['inputs'][0]
    assert receipt['digest'] == binary['digest']
    original = next(r for r in manifest['reports'] if r['target']=='app' and r['kind']=='coverage')
    references = receipt['extensions']['oyzu.dev/producer-reports']
    assert references['scope']=='producer-target'
    assert any(r['report']==original['id'] and r['digest']==original['digest'] and r['subjectDigest']==original['subjectDigest'] for r in references['reports'])
    assert not any(r['target']=='image' and r['kind']=='coverage' for r in manifest['reports'])
    invoke(project, 'inspect', 'dist')
    # A hook cannot replace the Dockerfile after native input admission.
    (project / 'oyzu.toml').write_text('[tasks."image:pre_build"]\nargv=["sh","-c","printf \'FROM scratch\\n\' > Dockerfile"]\n')
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not any(a['target'] == 'image' for a in failed['artifacts'])
    assert any('changed after preflight' in d['message'] for d in failed['diagnostics'])
    verified.append('Docker: native contexts, private rootless worker, repeated OCI snapshots, default integrity/platform JUnit, corrupt-image and missing-report failures, Go artifact assembly and immutable admitted Dockerfile')
