"""Native read-only quality checks with captured tools and project-owned excludes."""
import argparse
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT/'src/builders/python/runtime'
spec = importlib.util.spec_from_file_location('adapter', RUNTIME/'adapter.py')
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--wheels', type=Path, required=True)
    args = parser.parse_args()
    wheels = args.wheels.resolve()
    previous = Path.cwd()
    with tempfile.TemporaryDirectory(prefix='oyzu python quality ') as temporary:
        root = Path(temporary)
        try:
            os.chdir(root)
            # prepare_environment consumes a directory containing wheels/.
            dependencies = root/'dependencies'
            import shutil
            shutil.copytree(wheels, dependencies/'wheels')
            adapter.prepare_environment(dependencies)
            shutil.rmtree(dependencies)
            python = root/'.oyzu-build/venv'/('Scripts/python.exe' if os.name=='nt' else 'bin/python')
            def run(tool, *arguments, success=True):
                result = subprocess.run([str(python), '-I', str(RUNTIME/'quality.py'), tool, *arguments], capture_output=True, text=True, timeout=60)
                assert (result.returncode==0)==success, (result.stdout, result.stderr)
                return result
            app = root/'app.py'
            good = 'def health():\n    return {"status": "ok"}\n'
            app.write_text(good)
            (root/'.oyzu-build/canary.py').write_text('this is invalid Python !!!')
            (root/'excluded').mkdir()
            (root/'excluded/canary.py').write_text('this is invalid Python !!!')
            (root/'ignored').mkdir()
            (root/'ignored/canary.py').write_text('this is invalid Python !!!')
            (root/'ruff.toml').write_text('extend-exclude=["excluded", "ignored"]\nfix=true\n')
            run('ruff', 'check', '.', '--no-fix')
            run('ruff', 'format', '--check', '.')
            app.write_text('import json\n'+good)
            before = app.read_bytes()
            run('ruff', 'check', '.', '--no-fix', success=False)
            assert app.read_bytes()==before
            app.write_text(good)
            (root/'ruff.toml').unlink()
            config = root/'pyproject.toml'
            # Also preserve a native exclude, independently of extend-exclude.
            (root/'.flake8').write_text('[flake8]\nexclude=excluded\nextend-exclude=ignored\nignore=E501\nexit-zero=true\n')
            run('flake8', '.')
            app.write_text('import json\n\n\n'+good)
            before = app.read_bytes()
            result = run('flake8', '.', success=False)
            assert 'F401' in result.stdout and app.read_bytes()==before
            app.write_text(good)
            # Each regex retains native semantics, including trailing comments.
            for expression in ['"/ignored/"', '"(?x)/ignored/ # trailing comment"', '"""\n /ignored/ # trailing comment\n"""']:
                config.write_text('[tool.black]\nexclude="/excluded/"\nextend-exclude='+expression+'\n')
                run('black', '--check', '.')
                app.write_text('def health():\n return { "status":"ok" }\n')
                before = app.read_bytes()
                run('black', '--check', '.', success=False)
                assert app.read_bytes()==before
                run('black', '.')
                assert app.read_text()==good
        finally:
            os.chdir(previous)
    print('Python quality: offline native Ruff/Black/Flake8, preserved exclusions, private-state exclusion and read-only failures passed')


if __name__=='__main__':
    main()
