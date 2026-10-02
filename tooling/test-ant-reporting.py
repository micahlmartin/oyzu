"""Exercise native Ant assertion execution and JaCoCo against real class files."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--java-home', type=Path, required=True)
    parser.add_argument('--ant-home', type=Path, required=True)
    parser.add_argument('--jacoco-home', type=Path, required=True)
    args = parser.parse_args()
    java = args.java_home/'bin'/('java.exe' if os.name == 'nt' else 'java')
    javac = java.with_name('javac'+java.suffix)
    libraries = [args.ant_home/'lib/ant.jar', args.ant_home/'lib/ant-launcher.jar', args.jacoco_home/'jacocoant.jar']
    with tempfile.TemporaryDirectory(prefix='oyzu ant reporting ') as temporary:
        base = Path(temporary)
        classes = base/'adapter'
        classes.mkdir()
        classpath = os.pathsep.join(map(str, libraries))
        runtime = ROOT/'src/builders/java/ant/runtime'
        result = subprocess.run([str(javac), '-cp', classpath, '-d', str(classes), str(runtime/'AntTesting.java'), str(runtime/'AntCoverage.java')], capture_output=True, text=True, timeout=60)
        assert result.returncode == 0, result.stderr
        classpath = os.pathsep.join([str(classes), classpath])
        def run(project, name, target='test', success=True):
            junit, coverage = base/f'{name}-junit.xml', base/f'{name}-coverage.xml'
            result = subprocess.run([str(java), '-cp', classpath, 'AntTesting', str(junit), str(coverage), str(args.jacoco_home/'jacocoagent.jar'), '1.2.3-dev.g0123456789ab', target], cwd=project, capture_output=True, text=True, timeout=120)
            assert (result.returncode == 0) == success, (name, result.stdout, result.stderr)
            return junit, coverage, result
        for style in ['conventional', 'custom']:
            project = base/style
            shutil.copytree(ROOT/'examples/builds/java-ant'/style, project)
            target = 'test' if style == 'conventional' else 'verify-contract'
            before = (project/'build.xml').read_bytes()
            junit, coverage, _ = run(project, style, target)
            report = ET.parse(junit)
            assert len(report.findall('./testcase')) == 1
            assert report.find('./testcase').attrib['name'] == 'example.GreetingCheck'
            assert not report.findall('.//failure') and not report.findall('.//error')
            measured = ET.parse(coverage)
            assert [c.attrib['name'] for c in measured.findall('.//class')] == ['example/Greeting']
            assert int(measured.find("./counter[@type='INSTRUCTION']").attrib['covered']) > 0
            assert (project/'build.xml').read_bytes() == before
            # Stale reports must never be accepted or overwritten.
            original = junit.read_bytes()
            run(project, style, target, success=False)
            assert junit.read_bytes() == original
            check = project/'test/example/GreetingCheck.java'
            check.write_text(check.read_text().replace('Hello, Oyzu!', 'intentional failure'))
            # Delete only this temporary project's compiled harness so native Ant recompiles it.
            (project/'build/classes/example/GreetingCheck.class').unlink()
            junit, coverage, _ = run(project, style+'-failed', target, success=False)
            assert len(ET.parse(junit).findall('.//failure')) == 1
            assert int(ET.parse(coverage).find("./counter[@type='INSTRUCTION']").attrib['covered']) > 0
            junit, coverage, _ = run(project, style+'-missing', 'missing-target', success=False)
            assert len(ET.parse(junit).findall('.//error')) == 1 and not coverage.exists()
            build = project/'build.xml'
            build.write_text(build.read_text().replace('fork="true"', 'fork="false"'))
            junit, coverage, _ = run(project, style+'-nonfork', target, success=False)
            assert len(ET.parse(junit).findall('.//error')) == 1 and not coverage.exists()
        print('Native Ant Java assertion outcomes, JaCoCo application coverage, failures, stale outputs and custom targets passed')


if __name__ == '__main__':
    main()
