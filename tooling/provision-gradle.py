"""Pinned Gradle distribution for explicit native CI setup."""
import argparse
import hashlib
import io
from pathlib import Path
import zipfile

from provisioning import download

URL = 'https://downloads.gradle.org/distributions/gradle-8.14.3-bin.zip'
SHA256 = 'bd71102213493060956ec229d946beee57158dbd89d0e62b91bca0fa2c5f3531'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--destination', type=Path, required=True)
    args = parser.parse_args()
    data = download(URL, 256*1024*1024)
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise ValueError('Gradle distribution checksum mismatch')
    args.destination.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        archive.extractall(args.destination)
    (args.destination/'gradle-8.14.3/bin/gradle').chmod(0o755)
    (args.destination/'upstream.txt').write_text(
        f'Gradle 8.14.3\n{URL}\nSHA-256 {SHA256}\nComplete distribution retains upstream licenses/notices.\n', encoding='utf-8')
    print('Provisioned Gradle 8.14.3 for native CI probes')


if __name__ == '__main__':
    main()
