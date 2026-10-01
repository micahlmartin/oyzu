"""Native archive transport normalization preserves files, modes and long paths."""
import argparse
import hashlib
import io
from pathlib import Path
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tar-stream', type=Path, required=True)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='oyzu-archive-') as tmp:
        digests = []
        for index in range(2):
            archive = Path(tmp) / f'{index}.tgz'
            name = 'package/' + 'long-segment/' * 20 + 'tool.sh'
            with tarfile.open(archive, 'w:gz', format=tarfile.PAX_FORMAT) as tar:
                member = tarfile.TarInfo(name)
                member.mode = 0o755
                member.mtime = 1000000000 + index * 100
                member.uid = member.gid = 100 + index
                member.pax_headers = {'mtime': str(member.mtime) + '.5'}
                body = b'#!/bin/sh\nexit 0\n'
                member.size = len(body)
                tar.addfile(member, io.BytesIO(body))
                link = tarfile.TarInfo('package/link')
                link.type = tarfile.SYMTYPE
                link.linkname = name.removeprefix('package/')
                link.mtime = member.mtime
                tar.addfile(link)
            command = ['node', str(ROOT / 'src/builders/node/runtime/archive.mjs'), str(archive), str(args.tar_stream.resolve())]
            subprocess.run(command, check=True, capture_output=True, timeout=30)
            with tarfile.open(archive) as tar:
                assert tar.extractfile(name).read() == body
                assert tar.getmember(name).mode == 0o755 and tar.getmember(name).mtime == 0
                assert tar.getmember('package/link').linkname == link.linkname
                assert len(tar.getmembers()) == 2
            digests.append(hashlib.sha256(archive.read_bytes()).hexdigest())
            subprocess.run(command, check=True, capture_output=True, timeout=30)
            assert hashlib.sha256(archive.read_bytes()).hexdigest() == digests[-1]
            archive.write_bytes(b'corrupt archive')
            result = subprocess.run(command, capture_output=True, timeout=30)
            assert result.returncode != 0 and archive.read_bytes() == b'corrupt archive'
        assert digests[0] == digests[1]
    print('Node archive normalization: PAX paths, executable mode, links, payload, repeatability and malformed-input preservation passed')


if __name__ == '__main__':
    main()
