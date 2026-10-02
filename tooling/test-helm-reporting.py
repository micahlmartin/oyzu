"""Real native Helm validation: reports, failed values/templates and library charts."""
import argparse
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
validate = runpy.run_path(str(ROOT/'src/builders/helm/runtime/testing.py'))['validate']


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--helm', default='helm')
    parser.add_argument('--cli', type=Path)
    args = parser.parse_args()
    helm = str(Path(shutil.which(args.helm) or args.helm).resolve())
    with tempfile.TemporaryDirectory(prefix='oyzu-helm-reporting-') as temporary:
        base = Path(temporary)
        project = base/'project'
        shutil.copytree(ROOT/'examples/builds/helm-chart/project', project)
        subprocess.run([helm, 'dependency', 'build', str(project/'chart'), '--skip-refresh'], check=True)
        report, rendered = base/'junit.xml', base/'rendered.yaml'

        def check(chart, kind, expected):
            result = validate(chart, kind, report, rendered, helm)
            assert (result == 0) == (expected == 'passed'), (result, report.read_text())
            suite = ET.parse(report).getroot()
            assert suite.attrib['tests'] == '1' and len(suite.findall('testcase')) == 1
            assert suite.attrib['failures'] == ('1' if expected == 'failed' else '0')
            assert suite.find("properties/property[@name='coverage']").attrib['value'] == 'inapplicable'
            assert not suite.findall('.//coverage')
            assert rendered.exists() == (expected == 'passed' and kind == 'application')
            return suite

        check(project/'chart', 'application', 'passed')
        assert 'kind: Deployment' in rendered.read_text()
        values = project/'chart/values.yaml'
        original = values.read_text()
        values.write_text(original.replace('replicaCount: 1', 'replicaCount: 0'))
        failed = check(project/'chart', 'application', 'failed')
        assert 'replicaCount' in failed.find('testcase/failure').text
        values.write_text(original)
        template = project/'chart/templates/deployment.yaml'
        original = template.read_text()
        template.write_text('{{ fail "invalid <chart> & values" }}')
        failed = check(project/'chart', 'application', 'failed')
        assert 'invalid <chart> & values' in failed.find('testcase/failure').text
        template.write_text(original)
        check(project/'labels', 'library', 'passed')
        (project/'labels/values.schema.json').write_text(json.dumps({'type':'object','required':['requiredValue']}))
        check(project/'labels', 'library', 'failed')
        # Failure to start the real tool must replace previous success evidence.
        report.write_text('stale report')
        rendered.write_text('stale output')
        assert validate(project/'chart', 'application', report, rendered, str(base/'missing-helm')) != 0
        assert ET.parse(report).getroot().attrib['errors'] == '1' and not rendered.exists()
        tests = project/'chart/tests'
        tests.mkdir()
        (project/'chart/.helmignore').write_text('tests/\n')
        suite_file = tests/'deployment_test.yaml'
        source = (ROOT/'examples/builds/helm-chart/variants/deployment_test.yaml').read_text()
        suite_file.write_text(source)
        def cli_test(expected, directory=project):
            if args.cli:
                env = {**os.environ, 'PATH':str(Path(helm).parent)+os.pathsep+os.environ.get('PATH','')}
                result = subprocess.run([str(args.cli.resolve()),'-C',str(directory),'run','test'],env=env,capture_output=True,text=True,encoding='utf-8',timeout=120)
                assert (result.returncode==0)==expected, (result.stdout,result.stderr)
        if args.cli:
            result = subprocess.run([str(args.cli.resolve()),'-C',str(project),'run','list','--json'],env={**os.environ,'PATH':''},capture_output=True,text=True,timeout=30)
            assert result.returncode==0, result.stderr
            assert json.loads(result.stdout)['project:test']['argv'] == ['helm','unittest','--strict','chart']
        cli_test(True)
        unit_report = base/'unittest.xml'
        assert validate(project/'chart','application',report,rendered,helm,unit_report) == 0
        unit = ET.parse(unit_report)
        assert len(unit.findall('.//testcase')) == 1 and not unit.findall('.//failure')
        assert rendered.exists() and ET.parse(report).getroot().attrib['failures'] == '0'
        suite_file.write_text(source.replace('value: 1','value: 99'))
        cli_test(False)
        assert validate(project/'chart','application',report,rendered,helm,unit_report) != 0
        assert len(ET.parse(unit_report).findall('.//failure')) == 1
        # A native assertion failure does not rewrite the independent render result.
        assert ET.parse(report).getroot().attrib['failures'] == '0'
        suite_file.write_text('suite: [\n')
        assert validate(project/'chart','application',report,rendered,helm,unit_report) != 0
        if unit_report.exists():
            unit = ET.parse(unit_report)
            assert unit.findall('.//failure') or unit.findall('.//error') or not unit.findall('.//testcase')
        # A missing plugin cannot reuse the prior native report.
        assert validate(project/'chart','application',report,rendered,str(base/'missing-helm'),unit_report) != 0
        assert not unit_report.exists()
        suite_file.write_text((ROOT/'examples/builds/helm-chart/variants/deployment_snapshot_test.yaml').read_text())
        # Native unittest creates missing snapshots even without --update-snapshot.
        # A build must reject that success until the baseline is reviewed input.
        assert validate(project/'chart','application',report,rendered,helm,unit_report) != 0
        assert list((tests/'__snapshot__').glob('*.snap'))
        assert not ET.parse(unit_report).findall('.//failure')
        baseline = {p.name:p.read_bytes() for p in (tests/'__snapshot__').glob('*.snap')}
        assert validate(project/'chart','application',report,rendered,helm,unit_report) == 0
        assert baseline == {p.name:p.read_bytes() for p in (tests/'__snapshot__').glob('*.snap')}
        values.write_text(values.read_text().replace('replicaCount: 1','replicaCount: 2'))
        assert validate(project/'chart','application',report,rendered,helm,unit_report) != 0
        assert ET.parse(unit_report).findall('.//failure')
        assert baseline == {p.name:p.read_bytes() for p in (tests/'__snapshot__').glob('*.snap')}
        # Native subchart suites must be selected without any root test suite.
        subcharts = base/'subchart-only'
        shutil.copytree(ROOT/'examples/builds/helm-chart/variants/subchart-only', subcharts)
        if args.cli:
            listed = subprocess.run([str(args.cli.resolve()),'-C',str(subcharts),'run','list','--json'],
                                    env={**os.environ,'PATH':''},capture_output=True,text=True,timeout=30)
            assert listed.returncode==0, listed.stderr
            assert json.loads(listed.stdout)['project:test']['argv']==['helm','unittest','--strict','.']
        assert validate(subcharts,'application',report,rendered,helm,unit_report)==0
        unit = ET.parse(unit_report)
        assert len(unit.findall('.//testcase'))==1 and not unit.findall('.//failure')
        child_suite = subcharts/'charts/child/tests/configmap_test.yaml'
        cli_test(True, subcharts)
        child_suite.write_text(child_suite.read_text().replace('value: "42"','value: "99"'))
        assert validate(subcharts,'application',report,rendered,helm,unit_report)!=0
        assert len(ET.parse(unit_report).findall('.//failure'))==1
        cli_test(False, subcharts)
    print('Native Helm reports: rendering/schema checks, native unittest assertions/JUnit, subchart-only suites, malformed suites, failure evidence, library validation and stale-output rejection passed')


if __name__ == '__main__':
    main()
