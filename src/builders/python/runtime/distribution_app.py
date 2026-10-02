"""Assemble the native project's wheel and captured runtime closure as an app.

No checkout copying or second backend build. The selected console entrypoint
must agree with the wheel metadata; tests consume this exact archive's sources.
"""
import csv
import hashlib
import importlib.metadata
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys


def application():
    path = Path(__file__).with_name('python-app.py')
    if not path.exists():
        path = Path(__file__).with_name('application.py')
    spec = importlib.util.spec_from_file_location('oyzu_application', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def assemble(dependencies):
    app = application()
    adapter = app.runtime('python.py', 'adapter.py')
    wheels = list((app.STATE/'dist').glob('*.whl'))
    if len(wheels) != 1:
        raise ValueError('Expected one native application wheel')
    wheel = wheels[0]
    metadata = adapter.wheel_metadata(wheel)
    from pip._vendor.packaging.utils import canonicalize_name
    name = os.environ['OYZU_PYTHON_DISTRIBUTION']
    if canonicalize_name(metadata['Name']) != canonicalize_name(name) or metadata['Version'] != os.environ['OYZU_VERSION']:
        raise ValueError('Application wheel identity differs from plan')
    app.pure_wheel(wheel)
    project = app.STATE/'application-project'
    project.mkdir(exist_ok=False)
    adapter.run([sys.executable, '-I', '-m', 'pip', '--isolated', 'install', '--no-index', '--no-deps', '--no-compile', '--target', str(project), str(wheel)])
    distributions = list(importlib.metadata.distributions(path=[str(project)]))
    if len(distributions) != 1:
        raise ValueError('Expected one installed application distribution')
    distribution = distributions[0]
    script = os.environ['OYZU_PYTHON_ENTRYPOINT_NAME']
    expected = os.environ['OYZU_PYTHON_ENTRYPOINT_VALUE']
    entries = [entry for entry in distribution.entry_points if entry.group == 'console_scripts' and entry.name == script]
    if len(entries) != 1 or entries[0].value != expected:
        raise ValueError('Application console entrypoint differs from plan')
    packages = app.runtime_packages(dependencies)
    stage = app.STATE/'application'
    stage.mkdir(exist_ok=False)
    app.install_runtime(packages, dependencies, stage)
    app.remove_local_provenance(project)
    # pip-generated launchers bind temporary host interpreters. The zip launcher
    # below uses distribution metadata instead. Retain only payload RECORD rows.
    for root in [project, stage]:
        remove_launchers(root, app)
        for record in root.glob('*.dist-info/RECORD'):
            with record.open(newline='', encoding='utf-8') as stream:
                rows = [row for row in csv.reader(stream) if app.portable(row[0]) and (root/row[0]).is_file()]
            with record.open('w', newline='', encoding='utf-8') as stream:
                csv.writer(stream).writerows(rows)
    sources = []
    for path in sorted(project.rglob('*')):
        if path.is_symlink():
            raise ValueError('Symlink in installed application')
        if not path.is_file():
            continue
        relative = path.relative_to(project)
        destination = stage/relative
        if destination.exists():
            raise ValueError('Application wheel collides with a runtime dependency: '+relative.as_posix())
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        if path.suffix == '.py' and not any(part.endswith('.dist-info') for part in relative.parts):
            sources.append(relative.as_posix())
    if (stage/'__main__.py').exists() or (stage/app.MANIFEST).exists():
        raise ValueError('Application wheel uses a reserved archive path')
    launcher = (
        'from importlib.metadata import distribution\nimport sys\n'
        f'entries = distribution({name!r}).entry_points\n'
        f'entry = next(e for e in entries if e.group == "console_scripts" and e.name == {script!r})\n'
        'sys.exit(entry.load()())\n'
    )
    (stage/'__main__.py').write_text(launcher, encoding='utf-8', newline='\n')
    (app.STATE/'application-distributions.json').write_text(json.dumps(native_artifacts(app), sort_keys=True))
    app.write_archive(stage, sources, packages, distribution=name,
                      wheel={'filename':wheel.name, 'sha256':hashlib.sha256(wheel.read_bytes()).hexdigest()},
                      entrypoint={'name': script, 'value': expected})


def remove_launchers(root, app):
    for distribution in importlib.metadata.distributions(path=[str(root)]):
        for entry in distribution.entry_points:
            if entry.group not in {'console_scripts', 'gui_scripts'}:
                continue
            if not app.portable(entry.name) or '/' in entry.name:
                raise ValueError('Invalid installed console script name')
            for directory in ['bin', 'Scripts']:
                for suffix in ['', '.exe', '-script.py', '-script.pyw']:
                    path = root/directory/(entry.name+suffix)
                    if path.is_file():
                        path.unlink()
    # A distribution may legitimately contain a Python package called bin.
    # Remove empty launcher directories only, never the entire package tree.
    for directory in ['bin', 'Scripts']:
        path = root/directory
        if path.is_dir() and not any(path.iterdir()):
            path.rmdir()


def native_artifacts(app):
    artifacts = {}
    for path in (app.STATE/'dist').iterdir():
        if not path.is_file() or not path.name.endswith(('.whl', '.tar.gz')):
            raise ValueError('Unexpected native application artifact')
        artifacts[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    return artifacts


def main():
    app = application()
    adapter = app.runtime('python.py', 'adapter.py')
    if sys.argv[1:] == ['build']:
        adapter.build()
        assemble(Path('/dependencies'))
    elif sys.argv[1:] == ['package']:
        expected = json.loads((app.STATE/'application-distributions.json').read_text())
        if expected != native_artifacts(app):
            raise ValueError('Native application artifacts changed after assembly')
        app.package()
        adapter.package()
    elif len(sys.argv) == 3 and sys.argv[1] == 'assemble':
        assemble(Path(sys.argv[2]))
    else:
        raise ValueError('Unknown distribution application operation')


if __name__ == '__main__':
    main()
