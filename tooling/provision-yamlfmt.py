"""Explicit CI provisioning of pinned yamlfmt binaries and their upstream license."""
import argparse
import hashlib
import io
from pathlib import Path
import platform
import tarfile
import urllib.request

VERSION = '0.21.0'
ASSETS = {
    ('Windows', 'amd64'): ('Windows_x86_64', '07f80ce5d741eb4b0a9380ac78a19c7cb5bd44e2a9a47a5a04839e3ba54dd463'),
    ('Windows', 'arm64'): ('Windows_arm64', 'c1e64d1c72ca8986bc5b8c8edd4ec89f0627804e7e08f8de9f4b484cb5cad897'),
    ('Linux', 'x86_64'): ('Linux_x86_64', '1f300d9257b232bb3b541d7fb1b0e6b3c121bcbab381c86cd38cb8722be8a566'),
    ('Linux', 'aarch64'): ('Linux_arm64', '5b2689c963b177271330c5ce8ca7396751107e5a826be46f03d2cb9b6f0c7784'),
    ('Darwin', 'x86_64'): ('Darwin_x86_64', '060e943bcb8583c456810eb1ff4721b4f46c4a0c1a4432449d5dc3bbfe29a22b'),
    ('Darwin', 'arm64'): ('Darwin_arm64', '4b417ecb94339d57e4c122ecc948c1a00fe328b5853266de9806e652a92858fa'),
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--destination', type=Path, required=True)
    args = parser.parse_args()
    key = platform.system(), platform.machine().lower()
    name, digest = ASSETS[key]
    url = f'https://github.com/google/yamlfmt/releases/download/v{VERSION}/yamlfmt_{VERSION}_{name}.tar.gz'
    with urllib.request.urlopen(url, timeout=60) as response:
        body = response.read(32*1024*1024+1)
    if len(body) > 32*1024*1024 or hashlib.sha256(body).hexdigest() != digest:
        raise ValueError('yamlfmt release archive failed its pinned digest/size check')
    args.destination.mkdir(parents=True, exist_ok=True)
    binary = 'yamlfmt.exe' if key[0] == 'Windows' else 'yamlfmt'
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as archive:
        for name, output in [(binary, binary), ('LICENSE', 'yamlfmt-LICENSE')]:
            member = archive.getmember(name)
            if not member.isfile() or member.size > 64*1024*1024:
                raise ValueError('Invalid yamlfmt archive member')
            (args.destination/output).write_bytes(archive.extractfile(member).read())
    (args.destination/binary).chmod(0o755)
    (args.destination/'yamlfmt-source.txt').write_text(f'yamlfmt {VERSION}\n{url}\nSHA-256 {digest}\n')
    print(f'Provisioned yamlfmt {VERSION}, archive digest verified; license retained')


if __name__ == '__main__':
    main()
