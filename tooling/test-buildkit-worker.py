"""Native BuildKit worker contract probe; not compiled-Oyzu image acceptance.

Use a fresh rootless worker/store for every build. Its outer network is disabled,
and only the source copy and private output directory are mounted from the host.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
BUILDKIT = 'moby/buildkit:v0.25.0-rootless'
GO = 'golang:1.24-bookworm'
PROBE = '''package main
import ("fmt"; "net"; "os"; "strings"; "time")
func main() {
  if len(os.Args) != 2 { panic("missing host sentinel path") }
  if _, err := os.Stat(os.Args[1]); !os.IsNotExist(err) { panic("host sentinel is visible") }
  if _, err := os.Stat("/var/run/docker.sock"); !os.IsNotExist(err) { panic("host Docker socket is visible") }
  connection, err := net.DialTimeout("tcp", "1.1.1.1:443", 2*time.Second)
  if err == nil { connection.Close(); panic("RUN acquired external network access") }
  entries, err := os.ReadDir("/proc"); if err != nil { panic(err) }
  for _, entry := range entries {
    body, _ := os.ReadFile("/proc/"+entry.Name()+"/cmdline")
    if strings.Contains(string(body), "buildkitd") { panic("RUN can see the worker daemon process") }
  }
  fmt.Println("native RUN process/network/mount checks passed")
}
'''


def docker(*args, timeout=120, success=True):
    result = subprocess.run(['docker', *args], capture_output=True, text=True, timeout=timeout)
    if success:
        assert result.returncode == 0, result.stdout + result.stderr
    return result


def inspect_layout(path, sentinel):
    with tarfile.open(path) as archive:
        members = {m.name.removeprefix('./'): m for m in archive.getmembers() if m.isfile()}
        layout = json.load(archive.extractfile(members['oci-layout']))
        assert layout['imageLayoutVersion'] == '1.0.0'
        index = json.load(archive.extractfile(members['index.json']))

        def read(descriptor):
            algorithm, digest = descriptor['digest'].split(':')
            assert algorithm == 'sha256'
            body = archive.extractfile(members[f'blobs/sha256/{digest}']).read()
            assert len(body) == descriptor['size'] and hashlib.sha256(body).hexdigest() == digest
            assert sentinel not in body
            return body

        assert len(index['manifests']) == 1
        descriptor = index['manifests'][0]
        manifest = json.loads(read(descriptor))
        config = json.loads(read(manifest['config']))
        assert config['os'] == 'linux' and config['architecture'] == 'amd64'
        contents = {}
        for layer in manifest['layers']:
            with tarfile.open(fileobj=io.BytesIO(read(layer)), mode='r:*') as payload:
                for member in payload.getmembers():
                    if member.isfile():
                        body = payload.extractfile(member).read()
                        assert sentinel not in body
                        contents[member.name.removeprefix('./')] = body
        return descriptor['digest'], contents


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--evidence-dir', type=Path, required=True)
    args = parser.parse_args()
    evidence = args.evidence_dir.resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    image = json.loads(docker('image', 'inspect', BUILDKIT).stdout)[0]
    assert image['Config']['User'] in {'1000', '1000:1000'}, 'worker must start as an unprivileged UID'
    identity = image['Id']
    assert identity.startswith('sha256:')
    with tempfile.TemporaryDirectory(prefix='oyzu-buildkit-probe-') as temporary:
        base = Path(temporary)
        context = base / 'context'
        shutil.copytree(ROOT / 'examples/builds/docker-offline/project', context)
        # A synthetic sentinel stands in for an unmounted host secret. Its bytes
        # are never sent to the compiler, worker, Dockerfile or context.
        sentinel = ('oyzu-unmounted-' + uuid.uuid4().hex).encode()
        secret = base / 'host-sentinel'
        secret.write_bytes(sentinel)
        source = base / 'probe-source'
        source.mkdir()
        (source / 'main.go').write_text(PROBE)
        (source / 'go.mod').write_text('module example.invalid/worker-probe\n\ngo 1.24\n')
        output = base / 'probe-output'
        output.mkdir()
        docker('run', '--rm', '--pull=never', '--network=none', '--read-only', '--cap-drop=ALL',
               '--security-opt=no-new-privileges', '--tmpfs', '/tmp:rw,exec,nosuid,nodev',
               '--mount', f'type=bind,source={source},target=/src,readonly',
               '--mount', f'type=bind,source={output},target=/out', '--workdir', '/src',
               '--env', 'GOTOOLCHAIN=local', '--env', 'GOPROXY=off', '--env', 'GOSUMDB=off',
               '--env', 'CGO_ENABLED=0', '--env', 'GOCACHE=/tmp/go-cache', '--env', 'GOMAXPROCS=2',
               GO, 'go', 'build', '-trimpath', '-o', '/out/probe', '.', timeout=180)
        shutil.copyfile(output / 'probe', context / 'probe')
        (context / 'probe').chmod(0o755)
        original = (context / 'Dockerfile').read_text()
        (context / 'Dockerfile').write_text(original + '\nCOPY probe /probe\nRUN ' + json.dumps(['/probe', str(secret)]) + '\n')
        before = {p.relative_to(context).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in context.rglob('*') if p.is_file()}
        digests = []
        for attempt in range(2):
            name = 'oyzu-buildkit-probe-' + uuid.uuid4().hex
            destination = evidence / str(attempt)
            destination.mkdir()
            destination.chmod(0o777)  # Private bind for the rootless worker UID.
            try:
                docker('run', '--detach', '--pull=never', '--name', name,
                       '--network=none', '--memory=2g', '--cpus=2', '--pids-limit=256',
                       '--security-opt=seccomp=unconfined', '--security-opt=apparmor=unconfined',
                       '--security-opt=systempaths=unconfined',
                       '--mount', f'type=bind,source={context},target=/workspace,readonly',
                       '--mount', f'type=bind,source={destination},target=/output',
                       identity, '--oci-worker-snapshotter=native', '--oci-worker-gc=false')
                deadline = time.monotonic() + 45
                while True:
                    ready = docker('exec', name, 'buildctl', 'debug', 'workers', success=False, timeout=10)
                    if ready.returncode == 0:
                        break
                    state = json.loads(docker('inspect', name).stdout)[0]['State']
                    if not state['Running'] or time.monotonic() >= deadline:
                        raise AssertionError('rootless BuildKit worker did not become ready')
                    time.sleep(.25)
                inspection = json.loads(docker('inspect', name).stdout)[0]
                assert inspection['HostConfig']['NetworkMode'] == 'none'
                assert not inspection['HostConfig']['Privileged']
                assert '--oci-worker-no-process-sandbox' not in inspection['Args']
                assert not any(m['Destination'] == '/var/run/docker.sock' for m in inspection['Mounts'])
                result = docker('exec', name, 'buildctl', 'build', '--progress=plain', '--no-cache',
                                '--frontend', 'dockerfile.v0', '--local', 'context=/workspace',
                                '--local', 'dockerfile=/workspace', '--opt', 'platform=linux/amd64',
                                '--opt', 'force-network-mode=none', '--opt', 'build-arg:SOURCE_DATE_EPOCH=315532800',
                                '--output', 'type=oci,dest=/output/image.tar,name=oyzu/probe:0.0.0-dev.test,rewrite-timestamp=true',
                                timeout=180, success=False)
                (destination / 'build.stdout').write_text(result.stdout)
                (destination / 'build.stderr').write_text(result.stderr)
                assert result.returncode == 0, result.stdout + result.stderr
                digest, files = inspect_layout(destination / 'image.tar', sentinel)
                assert files['greeting.txt'] == (context / 'greeting.txt').read_bytes()
                assert files['probe'] == (context / 'probe').read_bytes()
                digests.append(digest)
            finally:
                logs = docker('logs', name, success=False)
                (destination / 'worker.log').write_text(logs.stdout + logs.stderr)
                docker('rm', '--force', '--volumes', name, success=False)
        assert digests[0] == digests[1], 'fresh workers produced different OCI image identities'
        assert before == {p.relative_to(context).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in context.rglob('*') if p.is_file()}
    record = {'worker': identity, 'ociDigest': digests[0], 'result': 'native rootless worker contract passed',
              'scope': 'worker/OCI probe only; compiled-Oyzu container build integration remains pending'}
    (evidence / 'summary.json').write_text(json.dumps(record, indent=2))
    print(json.dumps(record))


if __name__ == '__main__':
    main()
