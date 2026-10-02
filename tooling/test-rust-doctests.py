"""Real stable Cargo doctests and invocation-scoped JUnit, with no native output parser."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
ADAPTER = ROOT/'src/builders/rust/runtime/doctest.py'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cargo', default='cargo')
    args = parser.parse_args()
    # Rustup dispatches by executable name. Make relative paths independent of
    # the fixture cwd without following cargo symlinks into rustup/rustup-init.
    cargo = os.path.abspath(shutil.which(args.cargo) or args.cargo)
    with tempfile.TemporaryDirectory(prefix='oyzu Rust doctests ') as temporary:
        base = Path(temporary)
        project = base/'project'
        shutil.copytree(ROOT/'examples/builds/rust-workspace/project', project)
        env = {**os.environ, 'CARGO_TARGET_DIR':str(base/'target'), 'CARGO_NET_OFFLINE':'true'}
        metadata = subprocess.run([cargo,'metadata','--locked','--offline','--format-version','1'],
                                  cwd=project,env=env,capture_output=True,text=True,timeout=120)
        assert metadata.returncode==0, (metadata.returncode, metadata.stdout, metadata.stderr)
        data = json.loads(metadata.stdout)
        packages = [p['name'] for p in data['packages'] if p['id'] in data['workspace_members']
                    and any(t['doctest'] for t in p['targets'])]
        assert sorted(packages)==['example-core','example-macros']
        report = base/'reports/doctest.xml'

        def run(success, executable=cargo):
            result = subprocess.run([sys.executable,str(ADAPTER),'--cargo',executable,str(report),*packages],
                                    cwd=project,env=env,capture_output=True,text=True,timeout=180)
            assert (result.returncode==0)==success, (result.stdout,result.stderr,report.read_text() if report.exists() else 'missing report')
            suite = ET.parse(report).getroot()
            assert suite.attrib['tests']=='2'
            assert {c.attrib['name'] for c in suite.findall('testcase')}==set(packages)
            assert 'invocation' in suite.find("properties/property[@name='scope']").attrib['value']
            return suite

        before = {p.relative_to(project):p.read_bytes() for p in project.rglob('*') if p.is_file()}
        passed = run(True)
        assert passed.attrib['failures']=='0' and passed.attrib['errors']=='0'
        if os.name != 'nt':
            shim = base/'bin/cargo'
            shim.parent.mkdir()
            shim.symlink_to(cargo)
            assert run(True, str(shim)).attrib['failures']=='0'
        source = project/'core/src/lib.rs'
        good = source.read_text()
        source.write_text(good.replace('!example_core::greeting().is_empty()', 'example_core::greeting().is_empty()'))
        failed = run(False)
        assert failed.attrib['failures']=='1' and failed.attrib['errors']=='0'
        assert 'FAILED' in failed.find('testcase/failure').text
        source.write_text(good.replace('example_core::greeting()', 'example_core::missing_function()'))
        assert run(False).attrib['failures']=='1'
        source.write_bytes(before[Path('core/src/lib.rs')])
        assert run(False, str(base/'missing-cargo')).attrib['errors']=='2'
        assert not ET.parse(report).findall('.//failure'), 'stale assertion failure survived launch failure'
        assert before=={p.relative_to(project):p.read_bytes() for p in project.rglob('*') if p.is_file()}
    print('Native Cargo doctests passed: workspace selection, successful/failed assertions, compiler errors, independent package results, stale-report replacement and unchanged source')


if __name__ == '__main__':
    main()
