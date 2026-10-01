"""Compiled-CLI Gradle composite builds, native reports and deterministic JARs."""
import shutil
import subprocess
import zipfile


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'gradle-composite'
    shutil.copytree(root / 'examples/builds/java-gradle-multi-project/project', project)
    before = source_files(project)
    tasks = invoke(project, 'run', 'list', '--json')
    assert tasks['project:build']['build_stage'] and not tasks['project:test']['build_stage']
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    artifacts = manifest['artifacts']
    assert len(artifacts) == 3 and all('-dev.g' in a['version'] for a in artifacts)
    for artifact in artifacts:
        with zipfile.ZipFile(project / 'dist' / artifact['path']) as jar:
            assert any(n.endswith('.class') for n in jar.namelist())
    tests = [r for r in manifest['reports'] if r['kind'] == 'test']
    coverage = [r for r in manifest['reports'] if r['kind'] == 'coverage']
    assert len(tests) == 1 and tests[0]['summary']['passed'] == 1
    assert len(coverage) == 1 and coverage[0]['summary']['covered'] > 0
    assert coverage[0]['format'] == 'jacoco'
    assert [a['id'] for a in manifest['actions']] == ['project:build', 'project:package']
    invoke(project, 'inspect', 'dist')
    mounts = []
    for i, artifact in enumerate(artifacts):
        mounts += ['--mount', f"type=bind,source={project / 'dist' / artifact['path']},target=/app/{i}.jar,readonly"]
    result = subprocess.run(['docker', 'run', '--rm', '--network=none', *mounts, '--entrypoint', 'java',
                             'oyzu-toolchain/gradle:8.14.3-jdk17', '-cp', '/app/*', 'example.App'],
                            capture_output=True, text=True, timeout=60)
    assert result.returncode == 0 and result.stdout.strip() == 'Hello, Oyzu!', (result.stdout, result.stderr)
    repeated = invoke(project, 'build')
    assert repeated['planDigest'] == manifest['planDigest'], 'identical Gradle inputs changed the resolved plan'
    assert {a['id']: a['digest'] for a in repeated['artifacts']} == {a['id']: a['digest'] for a in artifacts}
    source = project / 'library/src/main/java/example/Greeting.java'
    source.write_text(source.read_text().replace('Hello, Oyzu!', 'Changed shared code'))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['artifacts']
    assert any(r['kind'] == 'test' and r['summary'].get('failed', 0) for r in failed['reports'])
    assert any(r['kind'] == 'coverage' and r['summary'].get('covered', 0) for r in failed['reports'])
    assert next(a for a in failed['actions'] if a['id'] == 'project:package')['status'] == 'blocked'
    invoke(project, 'inspect', 'dist')
    verified.append('Gradle composite: prepared repository files, fresh offline lifecycle, included-build snapshots, native JUnit/JaCoCo, repeatability and failed-test evidence')
