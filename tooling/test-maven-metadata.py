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
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "src/builders/java/maven/runtime"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--maven-home", type=Path, required=True)
    parser.add_argument("--java-home", type=Path)
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
                                    capture_output=True, text=True, timeout=120)
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
                   f"-Dmaven.repo.local={base / 'repository'}",
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
    print('Maven native metadata passed: reactor inheritance, dependency versions, output interpolation, source preservation and containment')


if __name__ == '__main__':
    main()
