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


def verify_target_platform(root, base, invoke, validate, source_files, verified):
    project = base/'docker-arm64-assembly'
    shutil.copytree(root/'examples/builds/docker-offline/project', project)
    (project/'build.yaml').write_text('image:\n  uses: docker/image\n  platform: linux/arm64\n')
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    artifact, = manifest['artifacts']
    digest, config, files = image_contents(project/'dist'/artifact['path'])
    assert artifact['kind'] == 'oci-image' and '-dev.g' in artifact['version']
    assert config['os'] == 'linux' and config['architecture'] == 'arm64'
    assert files['greeting.txt'] == (project/'greeting.txt').read_bytes()
    assert manifest['targets'][0]['platform'] == {'os':'linux', 'arch':'arm64'}
    plan = json.loads((project/'dist/plan.json').read_text())
    assert plan['tools'][0]['platform'] == {'os':'linux', 'arch':'amd64'}
    assert all(a['executionPlatform'] == {'os':'linux', 'arch':'amd64'} and
               a['targetPlatform'] == {'os':'linux', 'arch':'arm64'} for a in plan['actions'])
    dependency = json.loads((project/'dist/dependencies/image.json').read_text())
    assert dependency['manager']['platform'] == {'os':'linux', 'arch':'amd64'}
    assert dependency['targetPlatform'] == {'os':'linux', 'arch':'arm64'}
    assert dependency['extensions']['oyzu.dev/docker']['metadata']['selection']['targetPlatform'] == 'linux/arm64'
    assert dependency['extensions']['oyzu.dev/docker']['metadata']['targetExecution'] is False
    assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest']
    assert repeated['artifacts'][0]['ociDigest'] == digest
    assert repeated['artifacts'][0]['digest'] == artifact['digest']
    (project/'Dockerfile').write_text('FROM alpine:3.22\nCOPY greeting.txt /greeting.txt\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'base image platform differs' in failed['diagnostics'][0]['message']
    # An existing worker or host emulator is not authorization/evidence for
    # target execution. The native parser must identify RUN before actions.
    (project/'Dockerfile').write_text('FROM scratch\nRUN ["/application"]\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'native target execution' in failed['diagnostics'][0]['message']
    verified.append('Docker arm64 assembly on amd64: distinct worker/target facts, native OCI bytes, snapshot/JUnit/quality, repeatability, mismatched base and foreign RUN rejection; no target application execution claimed')


def verify_platform_matrix(root, base, invoke, validate, source_files, verified):
    project = base/'docker-platform-matrix'
    shutil.copytree(root/'examples/builds/docker-offline/project', project)
    (project/'build.yaml').write_text('image:\n  uses: docker/image\n  matrix:\n    platform: [linux/amd64, linux/arm64]\n')
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all(f'image:{stage}' in tasks for stage in ['build', 'test', 'lint', 'format-check'])
    invoke(project, 'build', 'image')
    manifest = validate(project/'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    assert len(manifest['artifacts']) == 3
    targets = {t['variant']['platform']: t['id'] for t in manifest['targets'] if 'platform' in t['variant']}
    assert set(targets) == {'linux/amd64', 'linux/arm64'}
    plan = json.loads((project/'dist/plan.json').read_text())
    selection = plan['extensions']['oyzu.dev/selection']
    assert selection['requested'] == ['image'] and set(selection['selected']) == set(targets.values())
    for platform, target in targets.items():
        artifact, = [a for a in manifest['artifacts'] if a['target'] == target]
        assert artifact['kind'] == 'oci-image' and '-dev.g' in artifact['version']
        assert artifact['variant'] == {'platform': platform}
        digest, config, contents = image_contents(project/'dist'/artifact['path'])
        assert digest == artifact['ociDigest']
        assert f"{config['os']}/{config['architecture']}" == platform
        assert contents['greeting.txt'] == (project/'greeting.txt').read_bytes()
        for stage in ['build', 'test', 'lint', 'format-check', 'package']:
            assert next(a for a in manifest['actions'] if a['id'] == f'{target}:{stage}')['status'] == 'succeeded'
        assert next(r for r in manifest['reports'] if r['target'] == target and r['kind'] == 'test')['summary']['passed'] == 2
        assert all(a['executionPlatform'] == {'os': 'linux', 'arch': 'amd64'} and
                   a['targetPlatform'] == {'os': 'linux', 'arch': platform.split('/')[1]}
                   for a in plan['actions'] if a['target'] == target)
    index, = [a for a in manifest['artifacts'] if a['kind'] == 'oci-index']
    assert index['variant'] == {} and '-dev.g' in index['version']
    aggregate, = [t for t in manifest['targets'] if t['builder'] == 'oyzu/oci-index']
    assert aggregate['platform'] is None and aggregate['id'] == index['target']
    operation = next(a for a in plan['actions'] if a['id'] == index['producer'])
    assert operation['targetPlatform'] is None
    assert set(operation['dependsOn']) == {f'{target}:package' for target in targets.values()}
    assert next(a for a in manifest['actions'] if a['id'] == index['producer'])['status'] == 'succeeded'
    with tarfile.open(project/'dist'/index['path']) as archive:
        files = {m.name.removeprefix('./'): m for m in archive.getmembers() if m.isfile()}
        descriptor, = json.load(archive.extractfile(files['index.json']))['manifests']
        assert descriptor['digest'] == index['ociDigest']
        data = archive.extractfile(files['blobs/sha256/'+descriptor['digest'].removeprefix('sha256:')]).read()
        assert hashlib.sha256(data).hexdigest() == descriptor['digest'].removeprefix('sha256:')
        assert len(data) == descriptor['size']
        image_index = json.loads(data)
        expected = {platform: next(a['ociDigest'] for a in manifest['artifacts'] if a['target'] == target) for platform, target in targets.items()}
        assert {f"{d['platform']['os']}/{d['platform']['architecture']}": d['digest'] for d in image_index['manifests']} == expected
        for image in image_index['manifests']:
            assert 'blobs/sha256/'+image['digest'].removeprefix('sha256:') in files
    invoke(project, 'inspect', str(project/'dist'/index['path']))
    invoke(project, 'inspect', 'dist')
    repeated = invoke(project, 'build', 'image')
    assert repeated['planDigest'] == manifest['planDigest']
    assert [(a['target'], a['digest']) for a in repeated['artifacts']] == [(a['target'], a['digest']) for a in manifest['artifacts']]
    # A real platform-specific hook failure must block the complete index while
    # retaining the successful peer's image and its independent evidence.
    (project/'oyzu.toml').write_text('[tasks."image:pre_test"]\nargv=["sh", "-ec", "test ! -f /out/'+targets['linux/arm64']+'/container/image.tar"]\n')
    invoke(project, 'build', 'image', success=False)
    failed = validate(project/'dist')
    assert not any(a['kind'] == 'oci-index' for a in failed['artifacts'])
    assert any(a['target'] == targets['linux/amd64'] for a in failed['artifacts'])
    assert not any(a['target'] == targets['linux/arm64'] for a in failed['artifacts'])
    assert next(a for a in failed['actions'] if a['id'] == index['producer'])['status'] == 'blocked'
    invoke(project, 'inspect', 'dist')
    # Target execution must not silently use the amd64 worker for ARM tests.
    native = base/'go-platform-matrix-admission'
    shutil.copytree(root/'examples/builds/container-variants/project', native)
    invoke(native, 'build', 'image', '--image', 'go=oyzu-toolchain/go:1.24-mod0.25.0', success=False)
    failed = validate(native/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert any('native target execution' in d['message'] for d in failed['diagnostics'])
    (native/'build.yaml').write_text((native/'build.yaml').read_text().replace('  path: api\n', '  path: api\n  platform: windows/amd64\n'))
    invoke(native, 'build', 'image', success=False)
    failed = validate(native/'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert any('conflicts with explicit producer api' in d['message'] for d in failed['diagnostics'])
    verified.append('Docker platform matrix: separate amd64/arm64 snapshots and complete OCI index, native JUnit/quality, stable content, unchanged sources, and no partial index after one platform fails; EX-027 rejects missing target execution and conflicting producer constraints')


def verify(root, base, invoke, validate, source_files, verified):
    verify_target_platform(root, base, invoke, validate, source_files, verified)
    verify_platform_matrix(root, base, invoke, validate, source_files, verified)
    from .platform_execution import verify_platform_execution
    verify_platform_execution(root, base, invoke, validate, source_files, verified)
    project = base / 'docker-offline'
    shutil.copytree(root / 'examples/builds/docker-offline/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert tasks['project:build']['build_stage']
    assert tasks['project:test']['availability']
    assert tasks['project:lint']['build_stage'] and not tasks['project:lint']['availability']
    assert tasks['project:format-check']['build_stage'] and not tasks['project:format-check']['availability']
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    for stage in ['lint', 'format-check']:
        assert next(a for a in manifest['actions'] if a['id']==f'project:{stage}')['status']=='succeeded'
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
    good = (project/'Dockerfile').read_text()
    original_ignore = (project/'.dockerignore').read_text()
    (project/'Dockerfile').write_text('FROM scratch\nCOPY    greeting.txt     /greeting.txt\n')
    unformatted = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and source_files(project)==unformatted
    assert next(a for a in failed['actions'] if a['id']=='project:format-check')['status']=='failed'
    (project/'Dockerfile').write_text('FROM scratch\nWORKDIR relative\nCOPY greeting.txt /greeting.txt\n')
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    assert next(a for a in failed['actions'] if a['id']=='project:lint')['status']=='failed'
    # Quality control files remain available to checks without entering COPY.
    (project/'.hadolint.yaml').write_text('ignored: [DL3000]\n')
    (project/'.editorconfig').write_text('root = true\n[Dockerfile]\ninsert_final_newline = false\n')
    (project/'.dockerignore').write_text('.hadolint.yaml\n.editorconfig\n')
    (project/'Dockerfile').write_text('FROM scratch\nWORKDIR relative\nCOPY . /context/')
    before = source_files(project)
    invoke(project, 'build')
    configured = validate(project/'dist')
    assert before==source_files(project)
    _, _, configured_files = image_contents(project/'dist'/configured['artifacts'][0]['path'])
    assert not any(name.endswith(('.hadolint.yaml','.editorconfig')) for name in configured_files)
    for name in ['.hadolint.yaml','.editorconfig']:
        (project/name).unlink()
    (project/'.dockerignore').write_text(original_ignore)
    (project/'Dockerfile').write_text(good)
    verified.append('Dockerfile native lint/read-only formatting gates block artifacts; ignored native quality configuration remains available without entering the image')
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

    provisioned = base/'docker-provisioned-base'
    shutil.copytree(root/'examples/builds/docker-offline/variants/provisioned-base',provisioned)
    before = source_files(provisioned)
    invoke(provisioned,'build')
    captured = validate(provisioned/'dist')
    assert captured['status']=='succeeded' and source_files(provisioned)==before
    image, = captured['artifacts']
    digest, config, files = image_contents(provisioned/'dist'/image['path'])
    assert 'etc/alpine-release' in files and files['greeting.txt']==b'captured base application\n'
    dependency=json.loads((provisioned/'dist/dependencies/project.json').read_text())
    binding, = dependency['extensions']['oyzu.dev/docker']['images']
    assert binding['reference']=='alpine:3.22' and binding['config'].startswith('sha256:')
    assert binding['manifest'].startswith('sha256:') and binding['tree_digest'].startswith('sha256:')
    repeated=invoke(provisioned,'build')
    assert repeated['planDigest']==captured['planDigest']
    assert repeated['artifacts'][0]['ociDigest']==digest and repeated['artifacts'][0]['digest']==image['digest']
    aliases=base/'docker-image-aliases'
    shutil.copytree(root/'examples/builds/docker-offline/variants/image-aliases',aliases)
    before=source_files(aliases)
    invoke(aliases,'build')
    equivalent=validate(aliases/'dist')
    assert equivalent['status']=='succeeded' and source_files(aliases)==before
    alias_image,=equivalent['artifacts']
    _,_,alias_files=image_contents(aliases/'dist'/alias_image['path'])
    assert alias_files['copied-release']==alias_files['etc/alpine-release']
    assert not any(name.startswith('base-etc/') for name in alias_files), 'temporary image mount leaked into a layer'
    alias_inputs=json.loads((aliases/'dist/dependencies/project.json').read_text())['extensions']['oyzu.dev/docker']['images']
    assert {i['reference'] for i in alias_inputs}=={'alpine:3.22','docker.io/library/alpine:3.22'}
    assert len({(i['name'],i['manifest'],i['config'],i['tree_digest']) for i in alias_inputs})==1
    verified.append('Docker native reference aliases bind one immutable context for FROM, external COPY and temporary image-backed RUN mounts')
    arguments=base/'docker-argument-base'
    shutil.copytree(root/'examples/builds/docker-offline/variants/argument-base',arguments)
    before=source_files(arguments)
    invoke(arguments,'build')
    expanded=validate(arguments/'dist')
    assert expanded['status']=='succeeded' and source_files(arguments)==before
    expanded_image,=expanded['artifacts']
    _,_,expanded_files=image_contents(arguments/'dist'/expanded_image['path'])
    assert expanded_files['etc/alpine-release']==alias_files['etc/alpine-release']
    expanded_inputs=json.loads((arguments/'dist/dependencies/project.json').read_text())['extensions']['oyzu.dev/docker']
    expanded_binding,=expanded_inputs['images']
    assert expanded_binding['reference']=='oyzu-fixture/alpine:amd64'
    assert expanded_inputs['metadata']['stages'][0]['base']==expanded_binding['reference']
    assert expanded_inputs['metadata']['selection']=={'targetPlatform':'linux/amd64','sourceDateEpoch':'315532800'}
    (arguments/'Dockerfile').write_text('ARG BASE\nFROM ${BASE}\n')
    invoke(arguments,'build',success=False)
    empty=validate(arguments/'dist')
    assert not empty['actions'] and not empty['artifacts']
    assert any('empty name' in d['message'] for d in empty['diagnostics'])
    verified.append('Docker global ARG defaults resolve through native expansion before capture; missing base defaults fail preflight')
    (provisioned/'Dockerfile').write_text('FROM example.invalid/oyzu/unprovisioned:1\n')
    invoke(provisioned,'build',success=False)
    missing=validate(provisioned/'dist')
    assert not missing['artifacts'] and not missing['actions']
    assert any('pre-provision' in d['message'] for d in missing['diagnostics'])
    verified.append('Docker provisioned bases: captured config/layer identity, native offline OCI contexts, isolated RUN, repeated snapshots and missing-input preflight failure')

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
