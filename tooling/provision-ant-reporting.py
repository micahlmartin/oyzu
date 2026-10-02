"""Pinned native CI prerequisites matching the Ant toolchain image."""
import argparse
import hashlib
import io
from pathlib import Path
import tarfile

from provisioning import download

ANT_URL = 'https://downloads.apache.org/ant/binaries/apache-ant-1.10.18-bin.tar.gz'
ANT_SHA512 = 'c510d744876d8da48dabc9495b023b6ec5284a0f18b40ee50d98ce099e791ca5c73e3cd4c80d3c90d3efc03f9620be43aad0a373ed47198284f7e3373e0db2c3'
CENTRAL = 'https://repo.maven.apache.org/maven2/'
ASSETS = [
    ('jacoco/jacocoant.jar', 'org/jacoco/org.jacoco.ant/0.8.13/org.jacoco.ant-0.8.13-nodeps.jar', 'a1705e3b1e14a86e7fe390192014af42076b730256130b4be40720e36ddd3f32'),
    ('jacoco/jacocoagent.jar', 'org/jacoco/org.jacoco.agent/0.8.13/org.jacoco.agent-0.8.13-runtime.jar', '47e700ccb0fdb9e27c5241353f8161938f4e53c3561dd35e063c5fe88dc3349b'),
    ('apache-ant-1.10.18/lib/junit-4.13.2.jar', 'junit/junit/4.13.2/junit-4.13.2.jar', '8e495b634469d64fb8acfa3495a065cbacc8a0fff55ce1e31007be4c16dc57d3'),
    ('apache-ant-1.10.18/lib/hamcrest-core-1.3.jar', 'org/hamcrest/hamcrest-core/1.3/hamcrest-core-1.3.jar', '66fdef91e9739348df7a096aa384a5685f4e875584cce89386a7a47251c4d8e9'),
]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--destination', type=Path, required=True)
    args = parser.parse_args()
    args.destination.mkdir(parents=True, exist_ok=True)
    body = download(ANT_URL, 64*1024*1024)
    if hashlib.sha512(body).hexdigest() != ANT_SHA512:
        raise ValueError('Ant archive checksum mismatch')
    with tarfile.open(fileobj=io.BytesIO(body), mode='r:gz') as archive:
        archive.extractall(args.destination, filter='data')
    for relative, artifact, digest in ASSETS:
        data = download(CENTRAL+artifact, 16*1024*1024)
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValueError('Ant reporting prerequisite checksum mismatch: '+artifact)
        output = args.destination/relative
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(data)
    (args.destination/'upstream.txt').write_text(
        f'Ant 1.10.18\n{ANT_URL}\nSHA-512 {ANT_SHA512}\n'
        + '\n'.join(f'{CENTRAL+artifact}\nSHA-256 {digest}' for _, artifact, digest in ASSETS)
        + '\nComplete upstream archives retain their license and notice files.\n', encoding='utf-8')
    print('Provisioned Ant 1.10.18, JaCoCo 0.8.13 and JUnit 4.13.2 for native CI probes')


if __name__ == '__main__':
    main()
