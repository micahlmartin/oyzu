"""Explicit CI/toolchain provisioning; never called by Oyzu project execution."""
import argparse
import hashlib
from pathlib import Path
import platform
import urllib.request

VERSION = '2.15.1'
ASSETS = {
    ('Linux', 'x86_64'): ('hadolint-linux-x86_64', 'c7187db94eeeeca956519a6af171adc31453941a1e777961f6e680f697c8c507'),
    ('Darwin', 'arm64'): ('hadolint-macos-arm64', '5c09f3213f8e40406abe048233d985eebef336d4a6a20021be47fadb6cf480a2'),
    ('Darwin', 'x86_64'): ('hadolint-macos-x86_64', 'ffe9bb18b23d5ed1eae50237aecdbb523d016e96da0bd4e7aa432040acfc3fde'),
    ('Windows', 'amd64'): ('hadolint-windows-x86_64.exe', '01d927294962b5387f9ead4f18679158452be4f17c765ad0bdffe5264b9c7b0a'),
}


def acquire(url, digest, destination):
    if destination.is_file() and hashlib.sha256(destination.read_bytes()).hexdigest() == digest:
        return
    with urllib.request.urlopen(url, timeout=60) as response:
        data = response.read(64 * 1024 * 1024 + 1)
    if len(data) > 64 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != digest:
        raise ValueError('Provisioned asset did not match its pinned SHA-256')
    temporary = destination.with_suffix('.pending')
    temporary.write_bytes(data)
    temporary.replace(destination)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--destination', type=Path, required=True)
    args = parser.parse_args()
    key = platform.system(), platform.machine().lower()
    if key not in ASSETS:
        raise ValueError(f'No pinned Hadolint asset for {key}')
    name, digest = ASSETS[key]
    args.destination.mkdir(parents=True, exist_ok=True)
    binary = args.destination/('hadolint.exe' if key[0] == 'Windows' else 'hadolint')
    acquire(f'https://github.com/hadolint/hadolint/releases/download/v{VERSION}/{name}', digest, binary)
    binary.chmod(0o755)
    acquire(f'https://raw.githubusercontent.com/hadolint/hadolint/v{VERSION}/LICENSE',
            '589ed823e9a84c56feb95ac58e7cf384626b9cbf4fda2a907bc36e103de1bad2', args.destination/'hadolint-LICENSE')
    (args.destination/'hadolint-source.txt').write_text(f'Hadolint v{VERSION}\nhttps://github.com/hadolint/hadolint/tree/v{VERSION}\n')
    print(f'Provisioned Hadolint {VERSION} with pinned checksum and upstream license')


if __name__ == '__main__':
    main()
