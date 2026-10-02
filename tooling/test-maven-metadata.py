"""Exercise the real Maven reactor model through Oyzu's native extension.

This is an offline native adapter check, not a claim of complete builder execution.
Tool installation is deliberately external to this script.
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import runpy
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "src/builders/java/maven/runtime"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--maven-home", type=Path, required=True)
    parser.add_argument("--java-home", type=Path)
    parser.add_argument("--test-lifecycle", action="store_true")
    parser.add_argument("--repository", type=Path)
    parser.add_argument("--acquire", action="store_true", help="Explicitly provision native test dependencies before offline checks")
    args = parser.parse_args()
    maven = args.maven_home.resolve()
    java = args.java_home.resolve() if args.java_home else Path(os.environ["JAVA_HOME"])
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="oyzu-maven-metadata-") as temporary:
        base = Path(temporary)
        classes = base / "classes"
        classes.mkdir()
        def run(command, cwd=base, env=None, success=True):
            result = subprocess.run([str(v) for v in command], cwd=cwd, env=env,
                                    capture_output=True, text=True, timeout=300)
            assert (result.returncode == 0) == success, (command, result.stdout, result.stderr)
            return result
        run([java / f"bin/javac{suffix}", "-cp", maven / "lib/*", "-d", classes, RUNTIME / "OyzuMetadata.java"])
        components = classes / "META-INF/plexus/components.xml"
        components.parent.mkdir(parents=True)
        shutil.copyfile(RUNTIME / "components.xml", components)
        extension = base / "oyzu-maven-metadata.jar"
        run([java / f"bin/jar{suffix}", "--create", "--file", extension, "-C", classes, "."])
        project = base / "project"
        shutil.copytree(ROOT / "examples/builds/java-maven-reactor/project", project)
        env = dict(os.environ, JAVA_HOME=str(java), MAVEN_SKIP_RC="true",
                   OYZU_MAVEN_WORKSPACE=str(project), OYZU_MAVEN_METADATA=str(base / "metadata.xml"))
        env.pop("MAVEN_ARGS", None)
        env.pop("MAVEN_OPTS", None)
        settings = base / "settings.xml"
        settings.write_text('<settings xmlns="http://maven.apache.org/SETTINGS/1.2.0"/>')
        launcher = maven / ("bin/mvn.cmd" if os.name == "nt" else "bin/mvn")
        command = [launcher, "-B", "-o", "-s", settings, "-gs", settings,
                   f"-Dmaven.repo.local={args.repository.resolve() if args.repository else base / 'repository'}",
                   f"-Duser.home={base / 'home'}", f"-Dmaven.ext.class.path={extension}", "validate"]
        before = {p.relative_to(project): p.read_bytes() for p in project.rglob('*') if p.is_file()}
        run(command, project, env)
        document = ET.parse(base / "metadata.xml")
        projects = {p.findtext("artifactId"): p for p in document.findall("project")}
        assert set(projects) == {"reactor", "library", "app"}
        assert all(p.findtext("version") == "0.1.0-SNAPSHOT" for p in projects.values())
        assert projects["app"].findtext("path") == "app"
        assert projects["library"].findtext("finalName") == "library-0.1.0-SNAPSHOT"
        dependencies = {d.findtext("artifactId"): d for d in projects['app'].findall('dependencies/dependency')}
        assert dependencies['library'].findtext('version') == '0.1.0-SNAPSHOT'
        assert dependencies['junit-jupiter'].findtext('version') == '5.11.4'
        assert projects['app'].findtext('testRoots/path') == 'app/src/test/java'
        assert before == {p.relative_to(project): p.read_bytes() for p in project.rglob('*') if p.is_file()}
        # Native model interpolation controls custom artifact filenames.
        pom = project / 'library/pom.xml'
        original = pom.read_text()
        pom.write_text(original.replace('</project>', '<build><finalName>custom-${project.artifactId}-${project.version}</finalName></build></project>'))
        run(command, project, env)
        projects = {p.findtext('artifactId'): p for p in ET.parse(base/'metadata.xml').findall('project')}
        assert projects['library'].findtext('finalName') == 'custom-library-0.1.0-SNAPSHOT'
        # Reject a native output location beyond the captured source root.
        pom.write_text(original.replace('</project>', '<build><directory>${project.basedir}/../../outside</directory></build></project>'))
        result = run(command, project, env, success=False)
        assert 'outside captured source' in result.stdout + result.stderr
        pom.write_text(original)
        if args.test_lifecycle:
            app = project/'app/pom.xml'
            variation = ROOT/'examples/builds/java-maven-reactor/variants/reporting'
            shutil.copyfile(variation/'app.pom.xml', app)
            unit = project/'app/src/test/java/example/AppTest.java'
            integration = unit.with_name('AppIT.java')
            shutil.copyfile(variation/'AppIT.java', integration)
            verify = [*command[:-1], 'verify']
            if args.acquire:
                run([arg for arg in verify if arg != '-o'], project, env)
            env['OYZU_MAVEN_TEST_PLAN'] = 'true'
            run(command, project, env)
            native_projects = ET.parse(base/'metadata.xml').findall('project')
            app_metadata = next(p for p in native_projects if p.findtext('artifactId') == 'app')
            assert {p.text for p in app_metadata.findall('testReports/directory')} == {
                'app/target/unit evidence', 'app/target/integration evidence'}
            reporting = runpy.run_path(str(RUNTIME/'reporting.py'))
            groups = reporting['inventory'](native_projects)
            (project/'.oyzu-maven').mkdir()
            env['OYZU_MAVEN_TEST_PLAN'] = 'false'
            previous = Path.cwd()
            try:
                os.chdir(project)
                for case in ['success', 'failed-integration', 'integration-only']:
                    output = project/'.oyzu-maven/reports'
                    if output.exists():
                        output.resolve().relative_to(project.resolve())
                        shutil.rmtree(output)
                    if case == 'failed-integration':
                        integration.write_text(integration.read_text().replace('Hello, Oyzu!', 'wrong result'))
                    elif case == 'integration-only':
                        integration.write_text(integration.read_text().replace('wrong result', 'Hello, Oyzu!'))
                        unit.unlink()
                        # Remove the stale compiled unit class as native clean would.
                        (project/'app/target/test-classes/example/AppTest.class').unlink()
                    reporting['prepare'](groups)
                    run(verify, project, env, success=case != 'failed-integration')
                    reporting['collect'](groups)
                    retained = list(output.rglob('TEST-*.xml'))
                    assert len(retained) == (2 if case == 'integration-only' else 3)
                    failures = sum(len(ET.parse(p).findall('.//failure')) for p in retained)
                    assert failures == (1 if case == 'failed-integration' else 0)
                    originals = {p.read_bytes() for paths in groups for value in paths for p in Path(value).glob('TEST-*.xml')}
                    assert {p.read_bytes() for p in retained} == originals
            finally:
                os.chdir(previous)
            env['OYZU_MAVEN_TEST_PLAN'] = 'true'
            configured = app.read_text()
            app.write_text(configured.replace('<execution>', '<execution><phase>none</phase>'))
            run(command, project, env)
            app_metadata = next(p for p in ET.parse(base/'metadata.xml').findall('project') if p.findtext('artifactId')=='app')
            assert [p.text for p in app_metadata.findall('testReports/directory')] == ['app/target/unit evidence']
            app.write_text(configured.replace('${project.build.directory}/unit evidence', '${project.basedir}/../../outside'))
            result = run(command, project, env, success=False)
            assert 'outside captured source' in result.stdout + result.stderr
            print('Maven lifecycle reports passed: native execution plan, custom directories, unit/integration XML, failure retention and integration-only modules')
    print('Maven native metadata passed: reactor inheritance, dependency versions, output interpolation, source preservation and containment')


if __name__ == '__main__':
    main()
