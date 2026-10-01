"""Native JDK interoperability and Ant metadata checks for embedded adapters."""
import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--java', default=shutil.which('java'), type=Path)
    parser.add_argument('--ant-home', type=Path)
    args = parser.parse_args()
    if not args.java:
        raise SystemExit('Provision a native JDK to check the Ant adapter')
    java = args.java.resolve()
    suffix = java.suffix
    def run(*command, success=True):
        result = subprocess.run([str(c) for c in command], capture_output=True, text=True, timeout=60)
        assert (result.returncode == 0) == success, (result.stdout, result.stderr)
        return result
    with tempfile.TemporaryDirectory(prefix='oyzu ant adapter ') as temporary:
        root = Path(temporary)
        (root/'Probe.java').write_text('public class Probe { public static void main(String[] args) { System.out.println("native JAR passed"); } }')
        run(java.with_name('javac'+suffix), '--release', '17', root/'Probe.java')
        raw = root/'original.jar'
        run(java.with_name('jar'+suffix), '--create', '--file', raw, '--main-class', 'Probe', '-C', root, 'Probe.class')
        helper = ROOT/'src/builders/java/ant/runtime/JarPackaging.java'
        first, second = root/'one.jar', root/'two.jar'
        version = '1.2.3-dev.g0123456789ab'
        run(java, helper, version, raw, first)
        run(java, helper, version, raw, second)
        assert first.read_bytes() == second.read_bytes()
        assert run(java, '-jar', first).stdout.strip() == 'native JAR passed'
        with zipfile.ZipFile(raw) as original, zipfile.ZipFile(first) as packaged:
            assert original.read('Probe.class') == packaged.read('Probe.class')
            assert f'Implementation-Version: {version}' in packaged.read('META-INF/MANIFEST.MF').decode()
            assert all(item.date_time == (1980, 1, 1, 0, 0, 0) for item in packaged.infolist())
        with zipfile.ZipFile(raw, 'a') as signed:
            signed.writestr('META-INF/SIGNATURE.SF', 'signature canary')
        run(java, helper, version, raw, root/'signed.jar', success=False)
        assert not (root/'signed.jar').exists()
        if args.ant_home:
            import os
            build = root/'build.xml'
            build.write_text('<project><property name="version" value="1.2.3"/><property name="output" value="dist"/><target name="jar"><jar destfile="${output}/app-${version}.jar"/></target></project>')
            before = build.read_bytes()
            classpath = os.pathsep.join(str(args.ant_home/'lib'/name) for name in ['ant.jar', 'ant-launcher.jar'])
            run(java, '--class-path', classpath, ROOT/'src/builders/java/ant/runtime/AntMetadata.java', build, root/'metadata.xml', version)
            metadata = ET.parse(root/'metadata.xml')
            assert metadata.getroot().attrib['version'] == version
            assert metadata.find('./target/jar').attrib['path'].endswith(f'/dist/app-{version}.jar')
            assert build.read_bytes() == before
    print('Native JAR payload, manifest, execution, repeatability and signed-input rejection checks passed')


if __name__ == '__main__':
    main()
