"""Real native Helm validation: reports, failed values/templates and library charts."""
import argparse
import json
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
    print('Native Helm reports: rendering/schema checks, failure evidence, escaped diagnostics, library validation and stale-output rejection passed')


if __name__ == '__main__':
    main()
