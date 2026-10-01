"""Native Ant conventional and custom task builds through Oyzu."""
import shutil
import subprocess
import zipfile


def verify(root, base, invoke, validate, source_files, verified):
    for name in ['conventional', 'custom']:
        project = base/f'ant-{name}'
        shutil.copytree(root/'examples/builds/java-ant'/name, project)
        before = source_files(project)
        listing = invoke(project, 'run', 'list', '--json')
        assert all(f'project:{task}' in listing for task in ['compile', 'build', 'archive'])
        plan = invoke(project, 'build', '--plan')
        assert plan == invoke(project, 'build', '--plan')
        invoke(project, 'build')
        manifest = validate(project/'dist')
        assert source_files(project) == before
        invoke(project, 'inspect', 'dist')
        assert not manifest['reports'], 'plain Java assertions must not become fabricated JUnit/coverage'
        artifact, = manifest['artifacts']
        assert '-dev.g' in artifact['version']
        with zipfile.ZipFile(project/'dist'/artifact['path']) as archive:
            assert 'example/Greeting.class' in archive.namelist()
            assert 'example/GreetingCheck.class' in archive.namelist()
            metadata = archive.read('META-INF/MANIFEST.MF').decode()
            assert f"Implementation-Version: {artifact['version']}" in metadata
        result = subprocess.run([
            'docker', 'run', '--rm', '--network=none', '--read-only',
            '--mount', f"type=bind,source={project/'dist'/artifact['path']},target=/app.jar,readonly",
            '--entrypoint', 'java', 'oyzu-toolchain/ant:1.10.18-jdk17', '-cp', '/app.jar', 'example.GreetingCheck',
        ], capture_output=True, text=True, timeout=60)
        assert result.returncode == 0, result.stderr
        test_id = 'project:test' if name=='conventional' else 'test'
        assert next(a for a in manifest['actions'] if a['id']==test_id)['status']=='succeeded'
        repeated = invoke(project, 'build')
        assert repeated['artifacts'][0]['digest'] == artifact['digest']
        verified.append(f'Ant {name}: native metadata, compile/test/jar, versioned executable JAR content, repeatability and unchanged source')

        if name=='conventional':
            test = project/'test/example/GreetingCheck.java'
            test.write_text(test.read_text().replace('Hello, Oyzu!', 'intentional failure'))
        else:
            build = project/'build.xml'
            build.write_text(build.read_text().replace('verify-contract', 'renamed-contract'))
        before = source_files(project)
        invoke(project, 'build', success=False)
        failed = validate(project/'dist')
        assert not failed['artifacts']
        assert source_files(project) == before
        assert next(a for a in failed['actions'] if a['id']==test_id)['status']=='failed'
        assert next(a for a in failed['actions'] if a['id']=='project:package')['status']=='blocked'
    verified.append('Ant assertion failure and missing custom target retain native diagnostics and block artifacts')

    project = base/'ant-version-property'
    shutil.copytree(root/'examples/builds/java-ant/conventional', project)
    build = project/'build.xml'
    build.write_text(build.read_text().replace('<target name="compile">', '<property name="version" value="1.2.3"/><target name="compile">').replace('dist/greeting.jar', 'dist/greeting-${version}.jar'))
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project/'dist')
    artifact, = manifest['artifacts']
    assert artifact['version'].startswith('1.2.3-dev.g')
    assert artifact['path'].endswith(f"greeting-{artifact['version']}.jar")
    assert artifact['name'] == 'jar-greeting'
    assert source_files(project) == before
    verified.append('Ant property-based output paths resolve natively and preserve a stable logical artifact name')
