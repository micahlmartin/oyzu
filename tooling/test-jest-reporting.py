"""Exercise the real Jest adapter with an explicitly provisioned native Jest CLI."""
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
    parser.add_argument('--jest-cli', type=Path, required=True)
    args = parser.parse_args()
    jest = args.jest_cli.resolve()
    with tempfile.TemporaryDirectory(prefix='oyzu-jest-reporting-') as temporary:
        base = Path(temporary)
        project = base / 'project'
        shutil.copytree(ROOT / 'tooling/fixtures/jest', project, ignore=shutil.ignore_patterns('node_modules'))
        report = base / 'reports/custom-tests.xml'
        coverage = base / 'reports/custom-coverage.lcov'
        env = dict(os.environ, OYZU_TEST_REPORT=str(report), OYZU_COVERAGE_REPORT=str(coverage))
        scope = []

        def run(success):
            if report.exists():
                report.unlink()
            if coverage.exists():
                coverage.unlink()
            result = subprocess.run(['node', str(ROOT / 'src/builders/node/runtime/jest.mjs'), 'node', str(jest), *scope],
                                    cwd=project, env=env, capture_output=True, text=True, encoding='utf-8', timeout=120)
            assert (result.returncode == 0) == success, result.stdout + result.stderr
            document = ET.parse(report)
            assert coverage.exists(), result.stdout + result.stderr
            return document

        document = run(True)
        cases = document.findall('.//testcase')
        assert len(cases) == 3 and len(document.findall('.//skipped')) == 2
        assert any(c.attrib['name'] == 'greets Oyzu & preserves <characters>' for c in cases)
        assert 'greeting.js' in coverage.read_text()
        assert any(line.startswith('DA:') and int(line.rsplit(',',1)[1]) > 0 for line in coverage.read_text().splitlines())
        source = project / 'test/greeting.test.js'
        original = source.read_text()
        source.write_text(original + "\ntest('actual failing assertion',()=>{expect(1).toBe(2)});\n")
        document = run(False)
        assert len(document.findall('.//failure')) == 1
        source.write_text(original)
        (project / 'test/broken.test.js').write_text("throw new Error('suite initialization failed');\n")
        document = run(False)
        assert len(document.findall('.//error')) == 1
        assert 'suite initialization failed' in document.find('.//error').text
        (project / 'test/broken.test.js').unlink()
        # Existing native reporters and config execute normally in the test phase.
        (project / 'custom-reporter.cjs').write_text("module.exports=class {onRunComplete(){require('node:fs').writeFileSync('custom-reporter-ran','yes')}};\n")
        (project / 'jest.config.cjs').write_text("module.exports={reporters:['default','<rootDir>/custom-reporter.cjs']};\n")
        run(True)
        assert (project / 'custom-reporter-ran').read_text() == 'yes'
        (project / 'test/skipped.test.js').write_text("describe.skip('skipped suite',()=>{test('native skipped test',()=>{})});\n")
        document = run(True)
        assert len(document.findall('.//skipped')) == 3
        # Root scoping must exclude members without replacing native ignores.
        for directory in ['packages/member', '.oyzu-build', 'ignored']:
            (project/directory).mkdir(parents=True)
            (project/directory/'failure.test.js').write_text("throw Error('excluded suite executed');\n")
        (project/'jest.config.cjs').write_text("module.exports={testPathIgnorePatterns:['/ignored/']};\n")
        module = (ROOT/'src/builders/node/runtime/npm-workspace-root.mjs').as_uri()
        version = json.loads((jest.parent.parent/'package.json').read_text())['version']
        scope = json.loads(subprocess.check_output(['node','--input-type=module','-e',f"import {{rootFrameworkArguments}} from {json.dumps(module)}; console.log(JSON.stringify(rootFrameworkArguments('jest',process.cwd(),[{{path:'packages/member'}}],{json.dumps(version)})));"], cwd=project, text=True))
        document = run(True)
        assert len(document.findall('.//testcase')) == 4 and len(document.findall('.//skipped')) == 3
        # Match native canonical test paths even when the caller uses an alias.
        alias = base/'aliased project'
        subprocess.run(['node','-e',"require('node:fs').symlinkSync(process.argv[1],process.argv[2],process.platform==='win32'?'junction':'dir')",str(project),str(alias)],check=True)
        project = alias
        scope = json.loads(subprocess.check_output(['node','--input-type=module','-e',f"import {{rootFrameworkArguments}} from {json.dumps(module)}; console.log(JSON.stringify(rootFrameworkArguments('jest',process.argv[1],[{{path:'packages/member'}}],{json.dumps(version)})));",str(alias)],cwd=project,text=True))
        assert len(run(True).findall('.//testcase')) == 4
    print('Native Jest reports passed: inferred tests, measured coverage, escaped names, skip/todo, assertion and suite failures, existing reporters')


if __name__ == '__main__':
    main()
