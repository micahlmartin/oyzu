"""Pinned Maven distribution for native CI probes; not product installation."""
import argparse
import hashlib
import io
from pathlib import Path
import tarfile

from provisioning import download

URL = 'https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.11/apache-maven-3.9.11-bin.tar.gz'
SHA512 = 'bcfe4fe305c962ace56ac7b5fc7a08b87d5abd8b7e89027ab251069faebee516b0ded8961445d6d91ec1985dfe30f8153268843c89aa392733d1a3ec956c9978'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--destination', type=Path, required=True)
    args = parser.parse_args()
    data = download(URL, 64*1024*1024)
    if hashlib.sha512(data).hexdigest() != SHA512:
        raise ValueError('Maven distribution checksum mismatch')
    args.destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        archive.extractall(args.destination, filter='data')
    (args.destination/'upstream.txt').write_text(
        f'Apache Maven 3.9.11\n{URL}\nSHA-512 {SHA512}\nComplete distribution retains upstream licenses/notices.\n', encoding='utf-8')
    print('Provisioned Maven 3.9.11 for native CI probes')


if __name__ == '__main__':
    main()
