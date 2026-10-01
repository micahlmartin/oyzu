"""Normalize transport metadata of a native Helm archive without changing files."""
import gzip
import io
from pathlib import Path, PurePosixPath
import sys
import tarfile


def normalize(path):
    entries = []
    names = set()
    total = 0
    with tarfile.open(path, 'r:gz') as source:
        for member in source:
            name = PurePosixPath(member.name)
            if (not member.isfile() or name.is_absolute() or '..' in name.parts
                    or '\\' in member.name or member.name in names):
                raise ValueError('unsafe or duplicate Helm archive entry')
            names.add(member.name)
            total += member.size
            if len(names) > 10000 or total > 64 * 1024 * 1024:
                raise ValueError('Helm archive exceeds normalization limit')
            entries.append((member.name, source.extractfile(member).read()))
    temporary = path.with_suffix(path.suffix + '.normalized')
    with temporary.open('xb') as raw:
        with gzip.GzipFile(filename='', mode='wb', fileobj=raw, mtime=0) as zipped:
            with tarfile.open(fileobj=zipped, mode='w', format=tarfile.PAX_FORMAT) as target:
                for name, body in sorted(entries):
                    info = tarfile.TarInfo(name)
                    info.mode = 0o644
                    info.size = len(body)
                    target.addfile(info, io.BytesIO(body))
    temporary.replace(path)


if __name__ == '__main__':
    for argument in sys.argv[1:]:
        path = Path(argument)
        if path.is_dir():
            for archive in sorted(path.rglob('*.tgz')):
                normalize(archive)
        else:
            normalize(path)
