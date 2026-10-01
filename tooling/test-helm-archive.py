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
