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


def chart_sources(root):
    # Direct tests now intentionally create a bundle/history. Preserve the
    # chart-byte invariant independently of those engine-owned outputs.
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*')
            if p.is_file() and p.relative_to(root).parts[0] not in {'dist', '.oyzu'}}


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
                manifest = json.loads((directory/'dist/manifest.json').read_text(encoding='utf-8'))
                assert manifest['status'] == ('succeeded' if expected else 'failed')
                inspected = subprocess.run([str(args.cli.resolve()),'-C',str(directory),'inspect','dist'],
                                           env=env,capture_output=True,text=True,timeout=120)
                assert inspected.returncode == 0, (inspected.stdout,inspected.stderr)
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
        assert not (tests/'__snapshot__').exists(), 'testing must leave source baselines unchanged'
        subprocess.run([helm, 'unittest', '--strict', '--update-snapshot', '.'], cwd=project/'chart', check=True)
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
        # Passing assertions do not replace native dependency/lint validation.
        subprocess.run([helm,'dependency','build',str(subcharts),'--skip-refresh'],check=True)
        subprocess.run([helm,'lint',str(subcharts),'--strict','--with-subcharts'],check=True)
        undeclared = base/'undeclared-child'
        shutil.copytree(subcharts,undeclared)
        parent = undeclared/'Chart.yaml'
        parent.write_text(parent.read_text().split('dependencies:')[0])
        lint = subprocess.run([helm,'lint',str(undeclared),'--strict','--with-subcharts'],capture_output=True,text=True)
        assert lint.returncode != 0 and 'missing these dependencies: child' in lint.stdout, (lint.stdout,lint.stderr)
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
        # Native package contents alone do not make the pinned plugin run tests.
        # Oyzu expands a private copy so packaged-only suites must run and fail.
        authored = base/'authored-packaged-only'
        shutil.copytree(ROOT/'examples/builds/helm-chart/variants/packaged-only', authored)
        authored_before = chart_sources(authored)
        subprocess.run([helm,'lint','--strict',str(authored)],check=True)
        assert validate(authored,'application',report,rendered,helm,unit_report)==0
        assert len(ET.parse(unit_report).findall('.//testcase'))==1
        cli_test(True, authored)
        assert authored_before == chart_sources(authored)
        authored_prepared = base/'authored-prepared'
        shutil.copytree(authored, authored_prepared, ignore=shutil.ignore_patterns('.oyzu', 'dist'))
        runpy.run_path(str(ROOT/'src/builders/helm/runtime/charts.py'))['expand'](authored_prepared)
        subprocess.run([helm,'dependency','build',str(authored_prepared),'--skip-refresh'],check=True)
        packed = base/'packaged-only'
        shutil.copytree(ROOT/'examples/builds/helm-chart/variants/subchart-only', packed)
        child = packed/'charts/child'
        archive_output = base/'archives'
        archive_output.mkdir()
        def package_child():
            subprocess.run([helm,'package',str(child),'--destination',str(archive_output)],check=True)
            archive = next(archive_output.glob('*.tgz'))
            shutil.copyfile(archive, packed/'charts'/archive.name)
        package_child()
        held = base/'held-child'
        shutil.move(str(child), held)
        if args.cli:
            listed = subprocess.run([str(args.cli.resolve()),'-C',str(packed),'run','list','--json'],
                                    env={**os.environ,'PATH':''},capture_output=True,text=True,timeout=30)
            assert listed.returncode == 0, listed.stderr
            assert json.loads(listed.stdout)['project:test']['argv']==['helm','unittest','--strict','.']
        original = chart_sources(packed)
        assert validate(packed,'application',report,rendered,helm,unit_report)==0
        assert len(ET.parse(unit_report).findall('.//testcase'))==1
        cli_test(True, packed)
        assert original == chart_sources(packed)
        shutil.move(str(held), child)
        suite = child/'tests/configmap_test.yaml'
        suite.write_text(suite.read_text().replace('value: "42"','value: "99"'))
        package_child()
        shutil.move(str(child), held)
        assert validate(packed,'application',report,rendered,helm,unit_report)!=0
        assert len(ET.parse(unit_report).findall('.//failure'))==1
        cli_test(False, packed)
        # Preparation expands native packaged dependencies before Helm's local build.
        prepared = base/'prepared-packed'
        shutil.copytree(packed,prepared,ignore=shutil.ignore_patterns('.oyzu', 'dist'))
        charts = runpy.run_path(str(ROOT/'src/builders/helm/runtime/charts.py'))
        charts['expand'](prepared)
        subprocess.run([helm,'dependency','build',str(prepared),'--skip-refresh'],check=True)
        empty = base/'empty-suites'
        shutil.copytree(ROOT/'examples/builds/helm-chart/variants/subchart-only',empty)
        (empty/'charts/child/tests/configmap_test.yaml').write_text('suite: empty\ntests: []\n')
        assert validate(empty,'application',report,rendered,helm,unit_report)!=0
    print('Native Helm reports: rendering/schema checks, native unittest assertions/JUnit, subchart-only suites, malformed suites, failure evidence, library validation and stale-output rejection passed')


if __name__ == '__main__':
    main()
