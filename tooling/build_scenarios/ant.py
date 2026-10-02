"""Native Ant conventional and custom task builds through Oyzu."""
import shutil
import subprocess
import zipfile
from .java_quality import verify as verify_quality
import xml.etree.ElementTree as ET


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
        assert any(r['kind']=='test' and r['summary']['passed']==1 for r in manifest['reports'])
        coverage, = [r for r in manifest['reports'] if r['kind']=='coverage']
        measured = ET.parse(project/'dist'/coverage['path'])
        assert [c.attrib['name'] for c in measured.findall('.//class')] == ['example/Greeting']
        assert int(measured.find("./counter[@type='INSTRUCTION']").attrib['covered']) > 0
        assert measured.find("./counter[@type='LINE']") is None
        native = measured.find("./counter[@type='INSTRUCTION']").attrib
        assert coverage['summary']=={'covered':int(native['covered']), 'total':int(native['covered'])+int(native['missed']), 'metric':'instructions'}
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
        verified.append(f'Ant {name}: native Java assertion JUnit, measured application JaCoCo coverage, versioned JAR content, repeatability and unchanged source')

        verify_quality(project, project/'src/example/Greeting.java', invoke, validate, source_files)
        verified.append(f'Ant {name}: native quality failures block artifacts without modifying source')

        if name=='conventional':
            config = project/'oyzu.toml'
            config.write_text('[tasks.test]\nargv=["sh","-c","true"]\n')
            invoke(project, 'build', success=False)
            missing = validate(project/'dist')
            assert not missing['artifacts']
            assert next(a for a in missing['actions'] if a['id']=='test')['status']=='failed'
            config.unlink()
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
        if name=='conventional':
            assert any(r['kind']=='test' and r['summary']['failed']==1 for r in failed['reports'])
            assert any(r['kind']=='coverage' and r['status']=='collected' for r in failed['reports'])
        else:
            assert any(r['kind']=='test' and r['summary']['failed']==1 for r in failed['reports'])
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

    project = base/'ant-junit'
    shutil.copytree(root/'tooling/fixtures/ant-junit', project)
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project/'dist')
    assert manifest['status']=='succeeded' and source_files(project)==before
    test, = [r for r in manifest['reports'] if r['kind']=='test']
    assert test['summary']=={'total':3, 'passed':2, 'failed':0, 'skipped':1}
    assert next(r for r in manifest['reports'] if r['kind']=='coverage')['summary']['metric']=='lines'
    report = ET.parse(project/'dist'/test['path'])
    assert {c.attrib['name'] for c in report.findall('.//testcase')}=={'positive','zero','negative'}
    coverage, = [r for r in manifest['reports'] if r['kind']=='coverage']
    measured = ET.parse(project/'dist'/coverage['path'])
    assert [c.attrib['name'] for c in measured.findall('.//class')]==['example/Calculator']
    branches = measured.find("./counter[@type='BRANCH']").attrib
    assert int(branches['covered'])>0 and int(branches['missed'])>0
    artifact, = manifest['artifacts']
    assert '-dev.g' in artifact['version']
    repeated = invoke(project, 'build')
    assert repeated['artifacts'][0]['digest']==artifact['digest']
    test = project/'test/example/CalculatorTest.java'
    test.write_text(test.read_text().replace('assertEquals(1,','assertEquals(99,'))
    before = source_files(project)
    invoke(project, 'build', success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts'] and source_files(project)==before
    assert any(r['kind']=='test' and r['summary']['failed']==1 and r['summary']['skipped']==1 for r in failed['reports'])
    assert any(r['kind']=='coverage' and r['status']=='collected' for r in failed['reports'])
    verified.append('Ant native JUnit: individual batch test/skip results, application-only branch coverage, repeated snapshot JARs and failure evidence despite native haltonfailure=false')
