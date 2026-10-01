"""Native Go metadata admission, workspace ownership and binary naming."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

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
        subprocess.run([go, 'build', '-trimpath', '-o', str(binary),
                        str(ROOT/'src/builders/go/runtime/metadata.go')], env=env, check=True)
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
    print('Native Go metadata: workspace, cgo compiler, multiple binaries, relocation, containment and test dependencies passed')


if __name__ == '__main__':
    main()
