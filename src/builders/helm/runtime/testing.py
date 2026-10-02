"""Native, local Helm validation with truthful JUnit outcomes; no cluster access."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET


def xml_text(value):
    # Native diagnostics can contain control bytes that XML 1.0 cannot represent.
    return ''.join(c if c in '\t\r\n' or 0x20 <= ord(c) <= 0xD7FF or
                   0xE000 <= ord(c) <= 0xFFFD or 0x10000 <= ord(c) <= 0x10FFFF
                   else '\ufffd' for c in value)


def unittest_adapter():
    import importlib.util
    spec = importlib.util.spec_from_file_location('oyzu_helm_charts', Path(__file__).with_name('helm-charts.py'))
    # Source-tree native probes use the owned file's source name.
    if not Path(spec.origin).is_file():
        spec = importlib.util.spec_from_file_location('oyzu_helm_charts', Path(__file__).with_name('charts.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate(chart, kind, report, rendered, helm='helm', unittest_report=None):
    report.parent.mkdir(parents=True, exist_ok=True)
    report.unlink(missing_ok=True)
    rendered.unlink(missing_ok=True)
    command = ([helm, 'lint', str(chart), '--strict', '--with-subcharts'] if kind == 'library'
               else [helm, 'template', 'oyzu-check', str(chart), '--dry-run=client'])
    name = ('library chart and values schema validation' if kind == 'library'
            else 'local template rendering and values schema validation')
    suite = ET.Element('testsuite', name='oyzu.helm.native-validation', tests='1', failures='0', errors='0', skipped='0')
    properties = ET.SubElement(suite, 'properties')
    for key, value in [('coverage', 'inapplicable'), ('coverage.reason', 'Chart assertions do not measure application source'),
                       ('scope', 'local chart validation; no Kubernetes API or runtime validation')]:
        ET.SubElement(properties, 'property', name=key, value=value)
    case = ET.SubElement(suite, 'testcase', classname='helm', name=name)
    code = 1
    # Output files avoid buffering arbitrarily large rendered charts in memory.
    with tempfile.TemporaryDirectory(prefix='oyzu-helm-test-') as temporary:
        stdout, stderr = Path(temporary)/'stdout', Path(temporary)/'stderr'
        env = dict(os.environ, KUBECONFIG=os.devnull)
        try:
            with stdout.open('wb') as out, stderr.open('wb') as err:
                result = subprocess.run(command, env=env, stdout=out, stderr=err, timeout=120)
            code = result.returncode
            if code:
                suite.set('failures', '1')
                failure = ET.SubElement(case, 'failure', message=f'Native Helm validation exited {code}')
                with stdout.open('rb') as out, stderr.open('rb') as err:
                    failure.text = xml_text((out.read(32768)+b'\n'+err.read(32768)).decode('utf-8', 'replace'))
            elif kind == 'application':
                rendered.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(stdout, rendered)
        except (OSError, subprocess.TimeoutExpired) as error:
            code = 1
            suite.set('errors', '1')
            ET.SubElement(case, 'error', message='Native Helm validation could not complete').text = xml_text(str(error))
        pending = report.with_suffix('.pending')
        ET.ElementTree(suite).write(pending, encoding='utf-8', xml_declaration=True)
        pending.replace(report)
    if unittest_report is not None:
        unittest_report = unittest_report.resolve()
        unittest_report.parent.mkdir(parents=True, exist_ok=True)
        unittest_report.unlink(missing_ok=True)
        try:
            result = unittest_adapter().unittest(chart, unittest_report, helm)
            if result:
                code = code or result
        except (OSError, ValueError, subprocess.TimeoutExpired) as error:
            print(f'Native Helm unittest could not complete: {error}', file=sys.stderr)
            code = code or 1
    return code if code >= 0 else 128-code


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('chart', type=Path)
    parser.add_argument('kind', choices=['application', 'library'])
    parser.add_argument('report', type=Path)
    parser.add_argument('rendered', type=Path)
    parser.add_argument('--unittest-report', type=Path)
    parser.add_argument('--host', action='store_true', help='Validate a private chart copy without local Oyzu state')
    args = parser.parse_args()
    if args.host:
        # A root chart also contains the live bundle lease and earlier dist
        # output. Native Helm loads chart files recursively; keep that engine
        # state out using the existing chart-copy boundary for host testing.
        with tempfile.TemporaryDirectory(prefix='oyzu-helm-host-') as temporary:
            copied = Path(temporary) / 'chart'
            unittest_adapter().copy_chart(args.chart.resolve(), copied)
            raise SystemExit(validate(copied, args.kind, args.report, args.rendered, unittest_report=args.unittest_report))
    raise SystemExit(validate(args.chart, args.kind, args.report, args.rendered, unittest_report=args.unittest_report))
