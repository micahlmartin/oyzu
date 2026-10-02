"""Contained chart expansion and native unittest execution; no downloads or cluster access."""
import argparse
import gzip
import hashlib
import io
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile
import tempfile
import xml.etree.ElementTree as ET


def redirected(path):
    return path.is_symlink() or bool(getattr(path.lstat(), "st_file_attributes", 0) & 0x400)


def portable(name):
    parts = name.split('/')
    for part in parts:
        stem = part.split('.')[0].upper()
        if (not part or part in ['.', '..'] or part.endswith(('.', ' '))
                or any(ord(c) < 32 or 0x7f <= ord(c) <= 0x9f or c in '\\:*?"<>|' for c in part)
                or stem in ['CON', 'PRN', 'AUX', 'NUL']
                or (len(stem) == 4 and stem[:3] in ['COM', 'LPT'] and stem[3] in '123456789')):
            return False
    return True


def expand(chart):
    """Only call on an invocation-private tree; replace archives with contained directories."""
    total = entries = 0
    pending = [(chart, 0)]
    while pending:
        current, depth = pending.pop()
        if depth > 16:
            raise ValueError('Helm archive/subchart depth exceeds 16')
        folder = current/'charts'
        if folder.is_symlink() or (folder.exists() and redirected(folder)):
            raise ValueError('Helm charts directory is a symbolic link')
        if not folder.exists():
            continue
        for item in sorted(folder.iterdir()):
            entries += 1
            if entries > 4096:
                raise ValueError('Helm archive entry count exceeds 4096')
            if item.name.startswith(('.', '_')):
                continue
            if redirected(item):
                raise ValueError('Helm subchart is a symbolic link')
            if item.is_dir():
                if (item/'Chart.yaml').is_file():
                    pending.append((item, depth+1))
                continue
            if item.suffix != '.tgz':
                continue
            if not item.is_file():
                raise ValueError('Helm archive must be a regular file')
            with item.open('rb') as stream:
                compressed = stream.read(4*1024*1024+1)
            if len(compressed) > 4*1024*1024:
                raise ValueError('Helm chart archive exceeds 4 MiB')
            with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
                raw = stream.read(64*1024*1024+1)
            total += len(raw)
            if total > 64*1024*1024:
                raise ValueError('Helm expanded archive inputs exceed 64 MiB')
            files, names, roots = [], set(), set()
            with tarfile.open(fileobj=io.BytesIO(raw), mode='r:') as archive:
                for member in archive:
                    entries += 1
                    name = member.name
                    if entries > 4096:
                        raise ValueError('Helm archive entry count exceeds 4096')
                    if not member.isfile() or not portable(name) or name.lower() in names:
                        raise ValueError('Unsafe or duplicate Helm archive entry')
                    names.add(name.lower())
                    path = PurePosixPath(name)
                    if len(path.parts) < 2:
                        raise ValueError('Helm archive requires one chart root')
                    roots.add(path.parts[0])
                    if member.size > 64*1024*1024:
                        raise ValueError('Helm archive member exceeds limit')
                    files.append((path, archive.extractfile(member).read()))
            if len(roots) != 1:
                raise ValueError('Helm archive requires one chart root')
            name, = roots
            if not any(str(path) == name+'/Chart.yaml' for path, _ in files):
                raise ValueError('Helm archive requires Chart.yaml')
            if name.startswith(('.', '_')) or name.lower() in {p.name.lower() for p in folder.iterdir()}:
                raise ValueError('Helm archive chart root collides with an existing or ignored entry')
            # All paths were admitted before any output is created. No extractall,
            # links, original permissions, archive ownership or timestamps are applied.
            destination = folder/name
            destination.mkdir()
            for relative, body in files:
                output = folder.joinpath(*relative.parts)
                output.parent.mkdir(parents=True, exist_ok=True)
                with output.open('xb') as stream:
                    stream.write(body)
            item.unlink()
            pending.append((destination, depth+1))


def snapshots(chart):
    identities = {}
    entries = total = 0
    def failed(error):
        raise error
    for directory, dirs, files in os.walk(chart, followlinks=False, onerror=failed):
        relative = Path(directory).relative_to(chart)
        for name in dirs + files:
            entries += 1
            if entries > 100000 or redirected(Path(directory)/name):
                raise ValueError('Helm snapshot inventory exceeds limits or contains symbolic links')
        if '__snapshot__' not in relative.parts:
            continue
        for name in files:
            path = Path(directory)/name
            if not path.is_file():
                raise ValueError('Helm snapshot baseline must be a regular file')
            with path.open('rb') as stream:
                content = stream.read(16*1024*1024+1)
            total += len(content)
            if len(content) > 16*1024*1024 or total > 64*1024*1024:
                raise ValueError('Helm snapshot baselines exceed limits')
            identities[(relative/name).as_posix()] = hashlib.sha256(content).hexdigest()
    return identities


def copy_chart(source, destination):
    entries = total = 0
    destination.mkdir()
    def failed(error):
        raise error
    for directory, dirs, files in os.walk(source, followlinks=False, onerror=failed):
        dirs[:] = sorted(d for d in dirs if d not in ['.git', '.oyzu', 'dist'])
        relative = Path(directory).relative_to(source)
        for name in dirs + files:
            entries += 1
            if entries > 100000 or redirected(Path(directory)/name):
                raise ValueError('Helm test input exceeds limits or contains symbolic links')
        for name in dirs:
            (destination/relative/name).mkdir()
        for name in sorted(files):
            path = Path(directory)/name
            if not path.is_file():
                raise ValueError('Helm test input must be a regular file')
            total += path.stat().st_size
            if total > 64*1024*1024:
                raise ValueError('Helm test inputs exceed 64 MiB')
            shutil.copyfile(path, destination/relative/name)


def unittest(chart, report=None, helm='helm'):
    with tempfile.TemporaryDirectory(prefix='oyzu-helm-unit-') as temporary:
        root = Path(temporary)
        chart = chart.resolve(strict=True)
        copied = root/'chart'
        copy_chart(chart, copied)
        expand(copied)
        baseline = snapshots(copied)
        report = report.resolve() if report else root/'unittest.xml'
        report.parent.mkdir(parents=True, exist_ok=True)
        report.unlink(missing_ok=True)
        result = subprocess.run([helm, 'unittest', '--strict', '--output-type', 'JUnit',
                                 '--output-file', str(report), '.'], cwd=copied,
                                env={**os.environ, 'KUBECONFIG':os.devnull}, timeout=120)
        if snapshots(copied) != baseline:
            raise ValueError('Helm unittest created or changed snapshot baselines; generate and review them before testing')
        if not report.is_file() or report.stat().st_size > 16*1024*1024:
            raise ValueError('Missing or oversized native Helm unittest report')
        try:
            document = ET.parse(report)
        except ET.ParseError as error:
            raise ValueError('Invalid native Helm unittest report') from error
        if not document.findall('.//testcase'):
            raise ValueError('Selected Helm unittest suites produced no test cases')
        return result.returncode or int(bool(document.findall('.//failure') or document.findall('.//error')))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('operation', choices=['prepare', 'test'])
    parser.add_argument('chart', type=Path)
    args = parser.parse_args()
    if args.operation == 'prepare':
        expand(args.chart)
    else:
        raise SystemExit(unittest(args.chart))
