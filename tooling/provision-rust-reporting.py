"""Explicit pinned CI prerequisites; no Oyzu product tool installation."""
import argparse
import hashlib
import io
from pathlib import Path
import platform
import tarfile
import zipfile

from provisioning import download

ASSETS = {
    'Windows': [
        ('nextest-rs/nextest', 'cargo-nextest-0.9.146', 'cargo-nextest-0.9.146-x86_64-pc-windows-msvc.zip', 'cargo-nextest.exe', '0fa689815c8157e4633225b6b173184b3d546eb6ffb754c3d0e6ea5973284a20'),
        ('taiki-e/cargo-llvm-cov', 'v0.9.1', 'cargo-llvm-cov-x86_64-pc-windows-msvc.zip', 'cargo-llvm-cov.exe', '656590cfa13bb7478595bbfdd3bc6bc075a4f7f3f0df5a439c1856cb03c5a4d2'),
    ],
    'Linux': [
        ('nextest-rs/nextest', 'cargo-nextest-0.9.146', 'cargo-nextest-0.9.146-x86_64-unknown-linux-gnu.tar.gz', 'cargo-nextest', '682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428'),
        ('taiki-e/cargo-llvm-cov', 'v0.9.1', 'cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz', 'cargo-llvm-cov', 'b3f68e625481fed9b16444174f3fa5ebcdbde4a1878803a35eabe2dcefcdc41a'),
    ],
    'Darwin': [
        ('nextest-rs/nextest', 'cargo-nextest-0.9.146', 'cargo-nextest-0.9.146-universal-apple-darwin.tar.gz', 'cargo-nextest', '39785160b3c2f6ed9a765049cf4fa79f3b39aa02eb7598a5a0e2a1a0b9ffb9a8'),
        ('taiki-e/cargo-llvm-cov', 'v0.9.1', 'cargo-llvm-cov-aarch64-apple-darwin.tar.gz', 'cargo-llvm-cov', '7e821a6c90c0884f79f759f5d9be50799ade67bb82515186d922f26032ef43b8'),
    ],
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--destination', type=Path, required=True)
    args = parser.parse_args()
    system, machine = platform.system(), platform.machine().lower()
    expected = {'Windows': {'amd64', 'x86_64'}, 'Linux': {'x86_64'}, 'Darwin': {'arm64', 'aarch64', 'x86_64'}}
    if machine not in expected.get(system, set()):
        raise ValueError(f'No pinned Rust reporting fixture for {system}/{machine}')
    args.destination.mkdir(parents=True, exist_ok=True)
    assets = list(ASSETS[system])
    if system == 'Darwin' and machine == 'x86_64':
        assets[1] = ('taiki-e/cargo-llvm-cov', 'v0.9.1', 'cargo-llvm-cov-x86_64-apple-darwin.tar.gz',
                     'cargo-llvm-cov', 'ddf949b82a63017441ea76aa068bdd7e2dde8055f47cd3ef736fdbbd633bc33e')
    for repository, version, archive_name, binary, digest in assets:
        url = f'https://github.com/{repository}/releases/download/{version}/{archive_name}'
        body = download(url, 128*1024*1024)
        if hashlib.sha256(body).hexdigest() != digest:
            raise ValueError(f'Native reporting archive digest mismatch: {archive_name}')
        if archive_name.endswith('.zip'):
            with zipfile.ZipFile(io.BytesIO(body)) as archive:
                data = archive.read(binary)
        else:
            with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as archive:
                member = archive.getmember(binary)
                if not member.isfile():
                    raise ValueError('Expected regular native executable')
                data = archive.extractfile(member).read()
        output = args.destination/binary
        output.write_bytes(data)
        output.chmod(0o755)
        (args.destination/(binary+'.source.txt')).write_text(f'{repository} {version}\n{url}\nSHA-256 {digest}\n', encoding='utf-8')
    print('Provisioned pinned nextest/llvm-cov for native CI tests')


if __name__ == '__main__':
    main()
