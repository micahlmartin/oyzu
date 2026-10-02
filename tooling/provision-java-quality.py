"""Explicit native test/image provisioning; never invoked by Oyzu installs/builds."""
import argparse
import hashlib
from pathlib import Path
import shutil
import urllib.request

ASSETS = {
    'google-java-format.jar': (
        'https://repo.maven.apache.org/maven2/com/google/googlejavaformat/google-java-format/1.24.0/google-java-format-1.24.0-all-deps.jar',
        '812f805f58112460edf01bf202a8e61d0fd1f35c0d4fabd54220640776ec57a1'),
    'checkstyle.jar': (
        'https://github.com/checkstyle/checkstyle/releases/download/checkstyle-10.21.4/checkstyle-10.21.4-all.jar',
        '3c1d94d6ecc83e02dff587c9ba5b6b4ec4fec38c7a958eb587efec9b28e2f318'),
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--destination', type=Path, required=True)
    args = parser.parse_args()
    args.destination.mkdir(parents=True, exist_ok=True)
    for name, (url, digest) in ASSETS.items():
        destination = args.destination/name
        if destination.is_file() and hashlib.sha256(destination.read_bytes()).hexdigest() == digest:
            continue
        with urllib.request.urlopen(url, timeout=60) as response:
            data = response.read(64 * 1024 * 1024 + 1)
        if len(data) > 64 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != digest:
            raise ValueError(f'Invalid pinned Java quality artifact: {name}')
        pending = destination.with_suffix('.pending')
        pending.write_bytes(data)
        pending.replace(destination)
    runtime = Path(__file__).resolve().parents[1]/'src/builders/java/runtime'
    for name in ['OyzuJavaQuality.java', 'checks.xml']:
        shutil.copyfile(runtime/name, args.destination/name)
    # Keep complete upstream distributions, including their embedded license/notice files.
    (args.destination/'upstream.txt').write_text(
        'google-java-format 1.24.0 (Apache-2.0)\nhttps://github.com/google/google-java-format/tree/v1.24.0\n'
        'Checkstyle 10.21.4 (LGPL-2.1-or-later)\nhttps://github.com/checkstyle/checkstyle/tree/checkstyle-10.21.4\n'
        'JARs are unchanged upstream distributions; their META-INF licenses/notices also cover bundled dependencies.\n')
    print('Provisioned pinned Java quality tools and owned adapter')


if __name__ == '__main__':
    main()
