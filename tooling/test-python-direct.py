"""Real Python manager execution and test-only bundles through the compiled CLI."""
import argparse
import importlib.util
import json
import os
import shutil
from pathlib import Path
import subprocess
import sys
import sysconfig
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--manager', choices=['pip','uv','poetry'], action='append')
    args = parser.parse_args()
    cli = args.cli.resolve()
    spec = importlib.util.spec_from_file_location('build_runner', ROOT/'tooling/test-build-scenarios.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    env = {**os.environ, 'PATH':str(Path(sys.executable).parent)+os.pathsep+os.environ.get('PATH','')}
    # Empty manager projects borrow explicitly provisioned reporter packages.
    # Native uv/Poetry still own interpreter/venv selection and execute pytest.
    env['PYTHONPATH'] = sysconfig.get_path('purelib')
    env['UV_PYTHON_DOWNLOADS'] = 'never'
    env['UV_OFFLINE'] = '1'
    env['POETRY_VIRTUALENVS_IN_PROJECT'] = 'true'
    env.pop('VIRTUAL_ENV', None)

    with tempfile.TemporaryDirectory(prefix='oyzu python direct ') as directory:
        base = Path(directory)
        for manager in args.manager or ['pip', 'uv', 'poetry']:
            project = base/manager
            project.mkdir()
            if manager=='pip':
                (project/'requirements.txt').write_text('')
            else:
                (project/'pyproject.toml').write_text('[project]\nname="direct-probe"\nversion="1.0.0"\nrequires-python=">=3.12"\n'
                    + ('[tool.poetry]\npackage-mode=false\n' if manager=='poetry' else '[tool.uv]\npackage=false\n'))
                command = ['uv','lock','--offline','--python',sys.executable] if manager=='uv' else ['poetry','lock']
                command[0] = shutil.which(command[0], path=env['PATH']) or command[0]
                setup = subprocess.run(command,cwd=project,env=env,capture_output=True,text=True,timeout=90)
                assert setup.returncode==0, (manager,setup.stdout,setup.stderr)
            (project/'application.py').write_text('def greeting(name):\n    if name:\n        return "Hi " + name\n    return "Hi"\n')
            (project/'unimported.py').write_text('def untouched():\n    return 42\n')
            test = 'from application import greeting\ndef test_greeting():\n    assert greeting("Ada") == "Hi Ada"\n'
            (project/'test_application.py').write_text(test)
            original_data = project/'.coverage'
            original_data.write_bytes(b'old data must never be read or overwritten')
            run_env = {**env, 'COVERAGE_FILE':str(original_data)}
            if manager=='uv':
                run_env['UV_PYTHON'] = sys.executable

            def invoke(*argv, code=0):
                result = subprocess.run([str(cli),'-C',str(project),'--json',*argv],env=run_env,capture_output=True,text=True,timeout=90)
                assert result.returncode==code, (manager,argv,result.returncode,result.stdout,result.stderr)
                return json.loads(result.stdout) if result.stdout.strip() else None

            outcomes = invoke('run','test','--','-k','greeting')
            assert outcomes[0]['exit_code']==0
            manifest = harness.validate(project/'dist')
            assert manifest['status']=='succeeded' and manifest['artifacts']==[]
            reports = {r['kind']:r for r in manifest['reports']}
            assert reports['test']['summary']['total']==1
            coverage = ET.parse(project/'dist'/reports['coverage']['path']).getroot()
            files = {c.attrib['filename']:c for c in coverage.findall('.//class')}
            assert all('test_' not in name for name in files), files
            assert any(name.endswith('application.py') for name in files), files
            unused = next(c for name,c in files.items() if name.endswith('unimported.py'))
            assert all(line.attrib['hits']=='0' for line in unused.findall('.//line'))
            assert original_data.read_bytes()==b'old data must never be read or overwritten'
            assert manifest['extensions']['oyzu.dev/invocation']['isolation']=='none'
            invoke('inspect','dist')

            # Native coverage settings and thresholds remain effective.
            (project/'.coveragerc').write_text('[run]\nbranch=true\n[report]\nfail_under=100\n')
            invoke('run','test',code=1)
            failed = harness.validate(project/'dist')
            assert failed['status']=='failed'
            assert next(r for r in failed['reports'] if r['kind']=='test')['summary']['failed']==0
            cov = next(r for r in failed['reports'] if r['kind']=='coverage')
            assert int(ET.parse(project/'dist'/cov['path']).getroot().attrib['branches-valid'])>0
            (project/'.coveragerc').unlink()

            # Native custom collection and a failing assertion retain evidence.
            (project/'checks').mkdir()
            (project/'checks/spec_application.py').write_text(test.replace('"Hi Ada"','"wrong"'))
            (project/'pytest.ini').write_text('[pytest]\ntestpaths=checks\npython_files=spec_*.py\n')
            invoke('run','test',code=1)
            failed = harness.validate(project/'dist')
            assert next(r for r in failed['reports'] if r['kind']=='test')['summary']['failed']==1
            (project/'checks/spec_application.py').unlink()
            invoke('run','test',code=5)
            empty = harness.validate(project/'dist')
            assert empty['status']=='failed'
            assert next(r for r in empty['reports'] if r['kind']=='test')['summary']['total']==0
            assert original_data.read_bytes()==b'old data must never be read or overwritten'
            if manager=='pip':
                # Exact shell shorthand, declared locations and hooks use the
                # same native report contract; custom bodies cannot omit it.
                (project/'pytest.ini').unlink()
                hook = "import os; from pathlib import Path; assert Path(os.environ['OYZU_TEST_REPORT']).is_file()"
                (project/'oyzu.toml').write_text('[tasks."project:test"]\nrun="python -m pytest"\nreports=[{kind="test",format="junit",path="native/reports/results.xml"}]\n[tasks."project:post_test"]\nargv='+json.dumps(['python','-c',hook])+'\n')
                invoke('run','test')
                assert harness.validate(project/'dist')['status']=='succeeded'
                assert (project/'native/reports/results.xml').is_file()
                (project/'oyzu.toml').write_text('[tasks."project:test"]\nargv=["python","-c","raise SystemExit(0)"]\n')
                invoke('run','test',code=1)
                missing = harness.validate(project/'dist')
                assert missing['status']=='failed' and all(r['status']=='invalid' for r in missing['reports'])
            print(f'{manager}: native test-only JUnit/application coverage, unimported source, arguments, thresholds, selection, failures, empty suite and fresh coverage data passed', flush=True)


if __name__=='__main__':
    main()
