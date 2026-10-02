"""Stable Cargo doctest gates; JUnit cases describe invocations, not doctest counts."""
import argparse
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET


def xml_text(value):
    return ''.join(c if c in '\t\r\n' or 0x20 <= ord(c) <= 0xD7FF or
                   0xE000 <= ord(c) <= 0xFFFD or 0x10000 <= ord(c) <= 0x10FFFF
                   else '\ufffd' for c in value)


def run(packages, report, cargo='cargo'):
    report.parent.mkdir(parents=True, exist_ok=True)
    report.unlink(missing_ok=True)
    if not packages or len(set(packages)) != len(packages):
        raise ValueError('Doctest package inventory must be nonempty and unique')
    suite = ET.Element('testsuite', name='oyzu.rust.doctest-invocations',
                       tests=str(len(packages)), failures='0', errors='0', skipped='0')
    properties = ET.SubElement(suite, 'properties')
    for name, value in [('scope', 'one native Cargo doctest invocation per enabled workspace package'),
                        ('coverage', 'not included in the nextest coverage report')]:
        ET.SubElement(properties, 'property', name=name, value=value)
    failures = errors = 0
    # Preserve bounded native diagnostics without parsing Cargo's human output.
    with tempfile.TemporaryDirectory(prefix='oyzu-cargo-doctest-') as temporary:
        for index, package in enumerate(packages):
            case = ET.SubElement(suite, 'testcase', classname='cargo.doctest', name=package)
            output = Path(temporary)/str(index)
            try:
                with output.open('wb') as stream:
                    result = subprocess.run([cargo, 'test', '--doc', '--locked', '--offline',
                                             '--package', package], stdout=stream,
                                            stderr=subprocess.STDOUT, timeout=120)
                with output.open('rb') as stream:
                    diagnostics = xml_text(stream.read(65536).decode('utf-8', 'replace'))
                ET.SubElement(case, 'system-out').text = diagnostics
                if result.returncode:
                    failures += 1
                    ET.SubElement(case, 'failure', message=f'Cargo doctests exited {result.returncode}').text = diagnostics
            except (OSError, subprocess.TimeoutExpired) as error:
                errors += 1
                ET.SubElement(case, 'error', message='Cargo doctests could not complete').text = xml_text(str(error))
    suite.set('failures', str(failures))
    suite.set('errors', str(errors))
    pending = report.with_suffix('.pending')
    ET.ElementTree(suite).write(pending, encoding='utf-8', xml_declaration=True)
    pending.replace(report)
    return int(bool(failures or errors))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--cargo', default='cargo')
    parser.add_argument('report', type=Path)
    parser.add_argument('packages', nargs='+')
    args = parser.parse_args()
    raise SystemExit(run(args.packages, args.report, args.cargo))
