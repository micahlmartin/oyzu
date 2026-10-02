"""Native Go metadata admission, workspace ownership and binary naming."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import hashlib
import threading
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--go', default='go')
    parser.add_argument('--cli', type=Path)
    args = parser.parse_args()
    go = shutil.which(args.go) or args.go
    with tempfile.TemporaryDirectory(prefix='oyzu-go-metadata-') as temporary:
        # macOS exposes /var through /private/var; Go workspace identity needs
        # the same physical root for cwd and an explicit GOWORK path.
        base = Path(temporary).resolve()
        env = dict(os.environ, GOENV='off', GOTOOLCHAIN='local', GOPROXY='off', GOSUMDB='off',
                   GOWORK='off', GO111MODULE='off', GOFLAGS='-p=2', GOMAXPROCS='2',
                   GOPATH=str(base/'gopath'), GOCACHE=str(base/'cache'), CGO_ENABLED='1')
        env['PATH'] = str(Path(go).resolve().parent) + os.pathsep + env['PATH']
        binary = base / ('metadata.exe' if os.name == 'nt' else 'metadata')
        runtime = base/'runtime'
        runtime.mkdir()
        for source in [ROOT/'src/builders/go/runtime/metadata.go', ROOT/'src/builders/go/runtime/acquisition.go', ROOT/'src/broker/runtime/transport.go']:
            shutil.copyfile(source, runtime/source.name)
        subprocess.run([go, 'build', '-trimpath', '-o', str(binary),
                        *map(str, runtime.glob('*.go'))], env=dict(env, CGO_ENABLED='0'), check=True)
        env['GO111MODULE'] = 'on'

        def capture(project, error=None):
            output = base/'metadata.json'
            output.unlink(missing_ok=True)
            result = subprocess.run([str(binary), str(output)], cwd=project, env=env,
                                    capture_output=True, text=True, timeout=120)
            if error:
                assert result.returncode and error in result.stderr, result.stderr
                assert not output.exists()
                return
            assert result.returncode == 0, result.stderr
            return json.loads(output.read_text())

        project = base/'workspace'
        shutil.copytree(ROOT/'examples/builds/go-workspace-cgo/project', project)
        first = capture(project)
        assert first['modules'] == ['cmd', 'math']
        assert first['moduleDependencies']['example.com/oyzu/command'] == ['example.com/oyzu/math']
        command_manifest = project/'cmd/go.mod'
        original_manifest = command_manifest.read_text()
        command_manifest.write_text(original_manifest.replace('require example.com/oyzu/math v0.0.0',''))
        assert capture(project)['moduleDependencies'] == first['moduleDependencies']
        command_manifest.write_text(original_manifest)
        assert first['patterns'] == ['./cmd/...', './math/...']
        assert [b['name'] for b in first['binaries']] == ['command']
        assert first['cgo'] and first['compiler'] and first['compilerTarget']
        native_env = dict(env, GOWORK=str(project/'go.work'), GOFLAGS='-mod=readonly -p=2')
        subprocess.run([go, 'test', *first['patterns']], cwd=project, env=native_env, check=True)
        if args.cli:
            for task in ['test', 'lint', 'build', 'format-check']:
                result = subprocess.run([str(args.cli.resolve()), '-C', str(project), '--json', 'run', task],
                                        env=native_env, capture_output=True, text=True, timeout=120)
                assert result.returncode == 0, (result.stdout, result.stderr)
                assert json.loads(result.stdout)[-1]['status'] == 'succeeded'
        assert capture(project) == first
        relocated = base/'relocated'
        shutil.copytree(project, relocated)
        assert capture(relocated) == first, 'metadata includes checkout paths'
        (project/'go.work').write_text('go 1.22\nuse ../relocated/cmd\n')
        capture(project, 'outside captured target')

        project = base/'binaries'
        shutil.copytree(ROOT/'examples/builds/materialize-selected-artifacts/project/tools', project)
        data = capture(project)
        assert [b['name'] for b in data['binaries']] == ['migrate', 'server']
        assert data['modules'] == ['.'] and not data['cgo']
        # Native go list must check test-only imports too, with no ambient cache.
        (project/'cmd/server/external_test.go').write_text('package main\nimport _ "example.invalid/missing"\n')
        capture(project, 'Go dependency metadata')
        (project/'cmd/server/external_test.go').unlink()
        with (project/'go.mod').open('a') as stream:
            stream.write('\nreplace example.invalid/missing => ../relocated/math\n')
        capture(project, 'outside captured target')
        verify_acquisition(base, binary, go, env)
    print('Native Go metadata: workspace, cgo compiler, multiple binaries, relocation, containment and test dependencies passed')


def verify_acquisition(base, binary, go, env):
    prefix = 'https://proxy.golang.org/github.com/google/uuid/@v/v1.6.0'
    responses = {}
    for extension in ['.mod', '.info', '.zip']:
        with urllib.request.urlopen(prefix+extension, timeout=30) as response:
            responses[prefix+extension] = response.read()
    spool = base/'broker'
    spool.mkdir()
    stop = threading.Event()
    seen, failures = [], []

    def serve():
        try:
            while not stop.is_set():
                for request in spool.glob('*.request'):
                    url = json.loads(request.read_text())['url']
                    seen.append(url)
                    request.unlink()
                    request.with_suffix('.body').write_bytes(responses.get(url, b'denied'))
                    response = request.with_suffix('.response-pending')
                    response.write_text(json.dumps({'status':200 if url in responses else 403, 'contentType':'application/octet-stream', 'sourceId':'go-public'}))
                    response.rename(request.with_suffix('.response'))
                time.sleep(.01)
        except Exception as error:
            failures.append(error)

    worker = threading.Thread(target=serve)
    worker.start()
    try:
        def capture(project, label, failure=None):
            output = base/label
            output.mkdir()
            result = subprocess.run([str(binary), str(output/'metadata.json'), str(output/'modules'), str(spool)],
                                    cwd=project, env=env, capture_output=True, text=True, timeout=180)
            if failure:
                assert result.returncode and failure in result.stderr, result.stderr
                assert not (output/'metadata.json').exists()
            else:
                assert result.returncode == 0, result.stderr
            return output

        empty = base/'empty-workspace'
        shutil.copytree(ROOT/'examples/builds/go-workspace-cgo/project', empty)
        empty_output = capture(empty, 'empty-modules')
        assert not seen and not json.loads((empty_output/'metadata.json').read_text())['dependencies'], seen
        project = base/'registry-project'
        shutil.copytree(ROOT/'tooling/fixtures/go-registry', project)
        def tree(path):
            return {p.relative_to(path).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                    for p in path.rglob('*') if p.is_file()}
        before = tree(project)
        first = capture(project, 'acquired-1')
        second = capture(project, 'acquired-2')
        assert tree(first) == tree(second), 'module snapshot depends on runtime location or fetch order'
        assert tree(project) == before, 'acquisition changed source or native checksums'
        metadata = json.loads((first/'metadata.json').read_text())
        dependency, = metadata['dependencies']
        assert dependency['name'] == 'github.com/google/uuid' and dependency['version'] == 'v1.6.0'
        assert dependency['digest'] == 'sha256:'+hashlib.sha256(responses[prefix+'.zip']).hexdigest()
        count = len(seen)
        replay = dict(env, GOMODCACHE=str(first/'modules'), GOPROXY='off', GOVCS='*:off', GOFLAGS='-mod=readonly -p=2')
        subprocess.run([go, 'test', './...'], cwd=project, env=replay, check=True)
        assert len(seen) == count and set(seen) <= set(responses)
        original = (project/'go.sum').read_text()
        (project/'go.sum').write_text(original.replace('NIvaJDMOsjHA8n1jAhLSgzrAzy1Hgr+hNrb57e+94F0=', 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA='))
        capture(project, 'bad-checksum', 'checksum mismatch')
        (project/'go.sum').write_text(original)
        (project/'go.sum').unlink()
        capture(project, 'missing-checksum', 'missing go.sum entry')
        assert not failures
        print('Native Go acquisition: scoped spool, unchanged checksums, repeatable cache, offline tests and invalid/missing checksum rejection passed')
    finally:
        stop.set()
        worker.join(timeout=5)


if __name__ == '__main__':
    main()
