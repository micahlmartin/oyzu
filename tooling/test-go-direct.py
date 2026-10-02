"""Native Go test-only evidence through the compiled CLI, without a daemon."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--go', default='go')
    args = parser.parse_args()
    cli = args.cli.resolve()
    go = Path(shutil.which(args.go) or args.go).resolve()
    spec = importlib.util.spec_from_file_location('build_runner', ROOT/'tooling/test-build-scenarios.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    with tempfile.TemporaryDirectory(prefix='oyzu go direct ') as temporary:
        base = Path(temporary).resolve()
        env = dict(os.environ, GOENV='off', GOTOOLCHAIN='local', GOPROXY='off', GOSUMDB='off',
                   GOWORK='off', GO111MODULE='on', GOFLAGS='-p=2', GOMAXPROCS='2', CGO_ENABLED='0',
                   GOPATH=str(base/'gopath'), GOCACHE=str(base/'cache'))
        env['PATH'] = str(go.parent)+os.pathsep+env.get('PATH', '')
        project = base/'single'
        project.mkdir()

        def module(path, name):
            path.mkdir(exist_ok=True)
            (path/'go.mod').write_text(f'module example.test/{name}\n\ngo 1.24.0\n')
            (path/'greeting.go').write_text('package greeting\nfunc Greeting(name string) string { if name == "" { return "Hi" }; return "Hi " + name }\n')
            (path/'greeting_test.go').write_text('package greeting\nimport "testing"\nfunc TestGreeting(t *testing.T) { if Greeting("Ada") != "Hi Ada" { t.Fatal("wrong greeting") } }\nfunc TestSkipped(t *testing.T) { t.Skip("deliberate") }\n')

        def invoke(*argv, code=0):
            result = subprocess.run([str(cli), '-C', str(project), '--json', *argv], env=env,
                                    capture_output=True, text=True, timeout=180)
            assert result.returncode == code, (argv, result.returncode, result.stdout, result.stderr)
            return json.loads(result.stdout) if result.stdout.strip() else None

        def bundle(status='succeeded'):
            manifest = harness.validate(project/'dist')
            assert manifest['status'] == status and manifest['artifacts'] == [], manifest
            assert manifest['extensions']['oyzu.dev/invocation']['isolation'] == 'none'
            invoke('inspect', 'dist')
            return {r['kind']: r for r in manifest['reports']}

        def config(body):
            (project/'oyzu.toml').write_text(body)

        def hook(body):
            config('[tasks."project:post_test"]\nargv='+json.dumps([sys.executable, '-c', body])+'\n')

        module(project, 'single')
        invoke('run', 'test')
        reports = bundle()
        assert reports['test']['summary'] == dict(passed=1, failed=0, skipped=1, total=2)
        assert reports['coverage']['format'] == 'go-cover'
        assert 0 < reports['coverage']['summary']['covered'] < reports['coverage']['summary']['total']
        coverage = (project/'dist'/reports['coverage']['path']).read_text()
        assert 'greeting.go:' in coverage and '_test.go:' not in coverage
        invoke('run', 'test', '--', '-run', 'TestGreeting')
        assert bundle()['test']['summary']['total'] == 1

        # The shared collector creates normalized JUnit before post hooks run.
        hook("import os; from pathlib import Path; import xml.etree.ElementTree as ET; p=Path(os.environ['OYZU_TEST_REPORT']); t=ET.parse(p); assert len(t.findall('.//testcase'))==2; p.write_text('<testsuite><testcase name=\"transformed\"/></testsuite>')")
        invoke('run', 'test')
        assert bundle()['test']['summary']['total'] == 1
        hook("import os; from pathlib import Path; Path(os.environ['OYZU_TEST_REPORT']).write_text('broken')")
        invoke('run', 'test', code=1)
        assert bundle('failed')['test']['status'] == 'invalid'
        config('[checks]\ncoverageMinimum=100\n')
        invoke('run', 'test', code=1)
        assert bundle('failed')['test']['summary']['failed'] == 0

        # An exact shell shorthand still gets native instrumentation.
        config('[tasks."project:test"]\nrun="go test ./..."\n')
        invoke('run', 'test')
        assert bundle()['test']['summary']['passed'] == 1
        (project/'oyzu.toml').unlink()
        test = project/'greeting_test.go'
        test.write_text(test.read_text().replace('!= "Hi Ada"', '!= "wrong"'))
        invoke('run', 'test', code=1)
        assert bundle('failed')['test']['summary']['failed'] == 1
        test.unlink()
        invoke('run', 'test')
        assert bundle()['test']['summary']['total'] == 0

        # A replacement producing similar stdout is not a trusted event source.
        custom = 'print(\'{"Package":"example.test/fake","Test":"Fake","Action":"pass"}\')'
        config('[tasks."project:test"]\nargv='+json.dumps([sys.executable, '-c', custom])+'\n')
        invoke('run', 'test', code=1)
        assert all(r['status'] == 'invalid' for r in bundle('failed').values())

        project = base/'workspace'
        project.mkdir()
        module(project/'api', 'api')
        module(project/'shared', 'shared')
        (project/'go.work').write_text('go 1.24.0\nuse (\n ./api\n ./shared\n)\n')
        invoke('run', 'test')
        reports = bundle()
        assert reports['test']['summary'] == dict(passed=2, failed=0, skipped=2, total=4)
        coverage = (project/'dist'/reports['coverage']['path']).read_text()
        assert 'example.test/api/greeting.go' in coverage and 'example.test/shared/greeting.go' in coverage
        invoke('run', 'test', '--', '-run', 'TestGreeting')
        assert bundle()['test']['summary']['total'] == 2
        test = project/'shared/greeting_test.go'
        test.write_text(test.read_text().replace('!= "Hi Ada"', '!= "wrong"'))
        invoke('run', 'test', code=1)
        reports = bundle('failed')
        assert reports['test']['summary']['failed'] == 1 and reports['test']['summary']['passed'] == 1
        print('Go direct tests: real module/workspace execution, JUnit/statement coverage, selection, failures, skips, empty suite, post-hook report visibility/transformation, malformed reports, thresholds and custom report obligations passed.')


if __name__ == '__main__':
    main()
