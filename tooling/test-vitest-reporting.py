"""Exercise native Vitest reports and configuration with provisioned dependencies."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--node-modules', type=Path, required=True)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='oyzu-vitest-reporting-') as temporary:
        base = Path(temporary)
        project = base / 'project'
        shutil.copytree(ROOT / 'tooling/fixtures/vitest', project, ignore=shutil.ignore_patterns('node_modules'))
        shutil.copytree(args.node_modules.resolve(), project / 'node_modules')
        report = base / 'reports/tests.xml'
        coverage = base / 'reports/coverage.lcov'
        env = dict(os.environ, OYZU_TEST_REPORT=str(report), OYZU_COVERAGE_REPORT=str(coverage))
        scope = []

        def run(success, reports=True):
            report.unlink(missing_ok=True)
            coverage.unlink(missing_ok=True)
            result = subprocess.run(['node', str(ROOT / 'src/builders/node/runtime/vitest.mjs'),
                                     'node', 'node_modules/vitest/vitest.mjs', 'run', *scope], cwd=project,
                                    env=env, capture_output=True, text=True, encoding='utf-8', timeout=120)
            assert (result.returncode == 0) == success, result.stdout + result.stderr
            if reports:
                assert report.exists() and coverage.exists(), result.stdout + result.stderr
                return ET.parse(report)
            assert not report.exists() and not coverage.exists(), 'stale reports were reused'
            return result

        document = run(True)
        assert len(document.findall('.//testcase')) == 3
        assert len(document.findall('.//skipped')) == 2
        assert any(c.attrib['name'] == 'greeting <&>' for c in document.findall('.//testcase'))
        assert 'src/greeting.js' in coverage.read_text().replace('\\', '/')
        assert any(line.startswith('DA:') and int(line.rsplit(',', 1)[1]) > 0 for line in coverage.read_text().splitlines())
        source = project / 'test/greeting.test.js'
        original = source.read_text()
        source.write_text(original + "\ntest('actual failure', () => expect(1).toBe(2));\n")
        assert len(run(False).findall('.//failure')) == 1
        source.write_text(original)
        broken = project / 'test/broken.test.js'
        broken.write_text("throw new Error('suite initialization failed');\n")
        document = run(False)
        assert document.findall('.//failure') or document.findall('.//error')
        broken.unlink()
        (project / 'reporter.mjs').write_text("import {writeFileSync} from 'node:fs'; export default class {onTestRunEnd(){writeFileSync('custom-reporter-ran','yes')}};\n")
        config = project / 'vitest.config.mjs'
        config.write_text("export default {test:{reporters:['default','./reporter.mjs'], coverage:{include:['src/**'], reporter:['json-summary']}}};\n")
        run(True)
        assert (project / 'custom-reporter-ran').read_text() == 'yes'
        for directory in ['packages/member', '.oyzu-build', 'ignored']:
            (project/directory).mkdir(parents=True)
            (project/directory/'failure.test.js').write_text("throw Error('excluded suite executed');\n")
        config.write_text("export default {test:{exclude:['**/node_modules/**','ignored/**'], coverage:{include:['src/**']}}};\n")
        module = (ROOT/'src/builders/node/runtime/npm-workspace-test-scope.mjs').as_uri()
        scope = json.loads(subprocess.check_output(['node','--input-type=module','-e',f"import {{frameworkArguments}} from {json.dumps(module)}; console.log(JSON.stringify(frameworkArguments('vitest',process.cwd(),[{{path:'packages/member'}}])));"], cwd=project, text=True))
        assert len(run(True).findall('.//testcase')) == 3
        for directory in ['packages', '.oyzu-build', 'ignored']:
            shutil.rmtree(project/directory)
        scope = []
        # Native Vite config fallback and threshold failure remain effective.
        config.rename(project / 'vite.config.mjs')
        run(True)
        (project / 'vite.config.mjs').unlink()
        (project / 'src/uncovered.js').write_text('export function unused() {\n  return 42;\n}\n')
        config.write_text("export default {test:{coverage:{include:['src/**'],thresholds:{lines:100}}}};\n")
        run(False)
        # Configuration failure must not reuse reports from the previous run.
        config.write_text("throw new Error('intentional config failure');\n")
        result = run(False, reports=False)
        assert 'intentional config failure' in result.stderr
        config.unlink()
        provider = project / 'node_modules/@vitest/coverage-v8'
        provider.rename(provider.with_name('coverage-v8-unavailable'))
        result = run(False, reports=False)
        assert 'must be captured before testing' in result.stderr
    print('Native Vitest: real JUnit/coverage, skip/todo, failed assertions, configured reporters, Vite config fallback, coverage thresholds and no stale evidence passed')


if __name__ == '__main__':
    main()
