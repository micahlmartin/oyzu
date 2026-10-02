"""Retain native lifecycle reports from the frozen Maven report-directory plan.

Surefire/Failsafe directories may legitimately be empty individually. A module's
combined group must still satisfy the shared collector's required JUnit contract.
XML is copied unchanged; parsing and result admission remain engine-owned.
"""
import hashlib
from pathlib import Path


def directory(value):
    path = Path(value)
    root = Path.cwd().resolve()
    if path.is_absolute() or '..' in path.parts or '\\' in value:
        raise ValueError('Maven report directory escapes captured workspace')
    resolved = path.resolve()
    resolved.relative_to(root)
    if resolved == root:
        raise ValueError('Maven report directory must be a child directory')
    if resolved != root/path or path.parts[0] == '.oyzu-maven':
        raise ValueError('Maven report directory is redirected or reserved')
    return path


def inventory(projects):
    groups = []
    for project in projects:
        reports = project.find('testReports')
        if reports is None:
            raise ValueError('Missing frozen Maven test report plan')
        groups.append(sorted({p.text for p in reports.findall('directory')}))
    return groups


def prepare(groups):
    output = Path('.oyzu-maven/reports')
    output.mkdir()
    for index, paths in enumerate(groups):
        (output/str(index)).mkdir()
    clear(groups)


def clear(groups):
    for paths in groups:
        for value in paths:
            for file in directory(value).glob('TEST-*.xml'):
                if file.is_symlink() or not file.is_file():
                    raise ValueError('Unsafe existing Maven report')
                file.unlink()


def collect(groups):
    for index, paths in enumerate(groups):
        for file, data in files(paths):
            identity = hashlib.sha256(file.as_posix().encode()).hexdigest()
            destination = Path('.oyzu-maven/reports')/str(index)/f'TEST-{identity}.xml'
            with destination.open('xb') as target:
                target.write(data)


def files(paths):
    remaining = 64 * 1024 * 1024
    for value in paths:
        for file in sorted(directory(value).glob('TEST-*.xml')):
            if file.is_symlink() or not file.is_file() or file.stat().st_size > 16 * 1024 * 1024:
                raise ValueError('Unsafe or oversized Maven report')
            with file.open('rb') as source:
                data = source.read(min(16 * 1024 * 1024, remaining) + 1)
            if len(data) > min(16 * 1024 * 1024, remaining):
                raise ValueError('Maven report group exceeds collection budget')
            remaining -= len(data)
            yield file, data
