"""Compiled CLI Python test selection with a provisioned native pytest."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    with tempfile.TemporaryDirectory(prefix='oyzu python tasks ') as temporary:
        project = Path(temporary)
        (project/'requirements.txt').write_text('')
        env = {**os.environ, 'PATH':str(Path(sys.executable).parent)+os.pathsep+os.environ.get('PATH','')}

        def run(*argv, code=0, extra_env=None):
            result = subprocess.run([str(cli),'-C',str(project),*argv],env={**env,**(extra_env or {})},capture_output=True,text=True,encoding='utf-8',timeout=60)
            assert result.returncode == code, (result.stdout,result.stderr)
            return result

        (project/'conftest.py').write_text("raise RuntimeError('static discovery executed Python')\n")
        tasks = json.loads(run('run','list','--json',extra_env={'PATH':''}).stdout)
        assert tasks['project:test']['availability'] is None
        assert tasks['project:test']['build_stage']
        assert tasks['project:test']['argv'] == ['python','-m','pytest']
        (project/'conftest.py').unlink()
        assert 'no tests ran' in run('run','test',code=5).stdout
        test = project/'test_root.py'
        test.write_text('def test_root():\n    assert 2 + 2 == 4\n')
        assert '1 passed' in run('run','test').stdout
        test.write_text("raise RuntimeError('native testpaths ignored')\n")
        checks = project/'checks'
        checks.mkdir()
        selected = checks/'spec_check.py'
        selected.write_text('def test_selected():\n    assert 2 + 2 == 4\n')
        (project/'pytest.ini').write_text('[pytest]\ntestpaths=checks\npython_files=spec_*.py\n')
        assert '1 passed' in run('run','test').stdout
        selected.write_text('def test_selected():\n    assert 2 + 2 == 5\n')
        assert '1 failed' in run('run','test',code=1).stdout
        selected.write_text('import pytest\n@pytest.mark.skip(reason="native skip")\ndef test_selected():\n    assert False\n')
        assert '1 skipped' in run('run','test').stdout
    print('Python CLI tasks passed: nonexecuting discovery, empty-suite exit, root tests, native configured paths/patterns, assertion failures and explicit skips')


if __name__ == '__main__':
    main()
