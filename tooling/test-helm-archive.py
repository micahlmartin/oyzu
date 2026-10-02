"""Archive adapter checks; optional native Helm interoperability probe."""
import argparse
import io
from pathlib import Path
import runpy
import shutil
import subprocess
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
normalize = runpy.run_path(str(ROOT/'src/builders/helm/runtime/archive.py'))['normalize']
expand = runpy.run_path(str(ROOT/'src/builders/helm/runtime/charts.py'))['expand']


def contents(path):
    with tarfile.open(path) as archive:
        return {member.name: archive.extractfile(member).read() for member in archive}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--helm', type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        archive = root/'test.tgz'
        with tarfile.open(archive, 'w:gz') as output:
            member = tarfile.TarInfo('example/values.yaml')
            body = b'replicaCount: 1\n'
            member.size = len(body)
            member.mtime = 999
            output.addfile(member, io.BytesIO(body))
        original = contents(archive)
        normalize(archive)
        assert contents(archive) == original
        first = archive.read_bytes()
        normalize(archive)
        assert archive.read_bytes() == first
        with tarfile.open(archive, 'w:gz') as output:
            output.addfile(tarfile.TarInfo('../escape'))
        try:
            normalize(archive)
        except ValueError:
            pass
        else:
            raise AssertionError('escaping entry accepted')
        for index, names in enumerate([
            ['child/Chart.yaml', '../escape'],
            ['child/Chart.yaml', 'child/Chart.yaml'],
            ['child/Chart.yaml', 'CHILD/chart.yaml'],
            ['child/Chart.yaml', 'other/file'],
            ['child/Chart.yaml', 'child/CON'],
            ['child/Chart.yaml', 'child/link'],
        ]):
            chart = root/f'unsafe-{index}'
            (chart/'charts').mkdir(parents=True)
            packed = chart/'charts/child.tgz'
            with tarfile.open(packed,'w:gz') as output:
                for name in names:
                    member = tarfile.TarInfo(name)
                    if name.endswith('/link'):
                        member.type = tarfile.SYMTYPE
                        member.linkname = '../../escape'
                    output.addfile(member,io.BytesIO(b''))
            before = packed.read_bytes()
            try:
                expand(chart)
            except ValueError:
                pass
            else:
                raise AssertionError(f'Unsafe chart archive admitted: {names}')
            assert packed.read_bytes() == before
            assert not (chart/'charts/child').exists()
        assert not (root/'escape').exists()
        packed = root/'contained'
        shutil.copytree(ROOT/'examples/builds/helm-chart/variants/packaged-only',packed)
        expand(packed)
        assert (packed/'charts/child/Chart.yaml').is_file()
        assert (packed/'charts/child/tests/configmap_test.yaml').is_file()
        assert not list((packed/'charts').glob('*.tgz'))
        if args.helm:
            project = root/'project'
            shutil.copytree(ROOT/'examples/builds/helm-chart/project', project)
            def helm(*arguments):
                subprocess.run([str(args.helm), *arguments], cwd=project, check=True, capture_output=True)
            helm('dependency', 'build', 'chart', '--skip-refresh')
            helm('package', 'chart', '--destination', str(root), '--version', '0.1.0-dev.g0123456789ab')
            chart = root/'greeting-0.1.0-dev.g0123456789ab.tgz'
            before = contents(chart)
            normalize(chart)
            assert contents(chart) == before
            first = chart.read_bytes()
            helm('lint', str(chart), '--strict', '--with-subcharts')
            helm('template', 'probe', str(chart))
            time.sleep(1.1)
            helm('package', 'chart', '--destination', str(root), '--version', '0.1.0-dev.g0123456789ab')
            normalize(chart)
            assert chart.read_bytes() == first
    print('Helm archive normalization checks passed' + ('; native Helm reload, lint, rendering and repeatability passed' if args.helm else ''))


if __name__ == '__main__':
    main()
