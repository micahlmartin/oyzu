"""Real Maven reactor acquisition, offline lifecycle and native bundle evidence."""
import shutil
import subprocess
import zipfile
from .java_quality import verify as verify_quality


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'maven-reactor'
    shutil.copytree(root / 'examples/builds/java-maven-reactor/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert all(f'project:{task}' in tasks for task in ['install', 'build', 'test'])
    assert tasks['project:build']['build_stage'] and not tasks['project:test']['build_stage']
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded'
    assert source_files(project) == before
    artifacts = manifest['artifacts']
    assert len(artifacts) == 5
    assert all('-dev.g' in a['version'] for a in artifacts)
    jars = [a for a in artifacts if a['path'].endswith('.jar')]
    assert len(jars) == 2
    for artifact in jars:
        with zipfile.ZipFile(project / 'dist' / artifact['path']) as jar:
            assert any(n.endswith('.class') for n in jar.namelist())
            metadata = next(n for n in jar.namelist() if n.endswith('/pom.properties'))
            assert f"version={artifact['version']}" in jar.read(metadata).decode()
    tests = [r for r in manifest['reports'] if r['kind'] == 'test']
    coverage = [r for r in manifest['reports'] if r['kind'] == 'coverage']
    assert len(tests) == 2 and all(r['summary']['passed'] > 0 for r in tests)
    assert len(coverage) == 2 and all(r['summary']['covered'] > 0 for r in coverage)
    assert all(r['format'] == 'jacoco' for r in coverage)
    assert len({r['id'] for r in tests + coverage}) == 4
    assert [a['id'] for a in manifest['actions']] == ['project:prepare', 'project:build', 'project:lint', 'project:format-check', 'project:package']
    invoke(project, 'inspect', 'dist')
    mounts = []
    for i, artifact in enumerate(jars):
        mounts += ['--mount', f"type=bind,source={project / 'dist' / artifact['path']},target=/app/{i}.jar,readonly"]
    result = subprocess.run(['docker', 'run', '--rm', '--network=none', *mounts, '--entrypoint', 'java',
                             'oyzu-toolchain/maven:3.9.11-jdk17', '-cp', '/app/*', 'example.App'],
                            capture_output=True, text=True, timeout=60)
    assert result.returncode == 0 and result.stdout.strip() == 'Hello, Oyzu!', (result.stdout, result.stderr)
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest'], 'identical Maven inputs changed the resolved plan'
    assert {a['id']: a['digest'] for a in repeated['artifacts']} == {a['id']: a['digest'] for a in artifacts}
    # Changing shared code must fail its native tests and block all exports.
    source = project / 'library/src/main/java/example/Greeting.java'
    verify_quality(project, source, invoke, validate, source_files)
    verified.append('maven: native lint/format failures retain test evidence, block snapshots and leave source unchanged')
    source.write_text(source.read_text().replace('Hello, Oyzu!', 'Changed shared code'))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['artifacts']
    assert any(r['kind'] == 'test' and r['summary'].get('failed', 0) for r in failed['reports'])
    assert next(a for a in failed['actions'] if a['id'] == 'project:package')['status'] == 'blocked'
    invoke(project, 'inspect', 'dist')
    verified.append('Maven reactor: scoped repository, one offline lifecycle, module JAR/POM snapshots, Surefire/JaCoCo, repeatability and failed-test evidence')

    project = base/'maven-native-reports'
    shutil.copytree(root/'examples/builds/java-maven-reactor/project', project)
    variation = root/'examples/builds/java-maven-reactor/variants/reporting'
    shutil.copyfile(variation/'app.pom.xml', project/'app/pom.xml')
    integration = project/'app/src/test/java/example/AppIT.java'
    shutil.copyfile(variation/'AppIT.java', integration)
    for case in ['both', 'failed-integration', 'integration-only']:
        if case == 'failed-integration':
            integration.write_text(integration.read_text().replace('Hello, Oyzu!', 'wrong result'))
        elif case == 'integration-only':
            integration.write_text(integration.read_text().replace('wrong result', 'Hello, Oyzu!'))
            (project/'app/src/test/java/example/AppTest.java').unlink()
        before = source_files(project)
        invoke(project, 'build', success=case != 'failed-integration')
        manifest = validate(project/'dist')
        assert source_files(project) == before
        reports = [r for r in manifest['reports'] if r['kind']=='test']
        assert len(reports) == (2 if case=='integration-only' else 3)
        assert sum(r['summary'].get('failed',0) for r in reports) == (1 if case=='failed-integration' else 0)
        coverage = [r for r in manifest['reports'] if r['kind']=='coverage']
        assert len(coverage)==2 and all(r['summary']['covered'] > 0 for r in coverage)
        if case == 'failed-integration':
            assert not manifest['artifacts']
        else:
            assert len(manifest['artifacts'])==5
            assert all('-dev.g' in a['version'] for a in manifest['artifacts'])
    verified.append('Maven native report plan retains custom Surefire/Failsafe XML and coverage; integration-only modules work and integration failures block artifacts')
