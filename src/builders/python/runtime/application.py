"""Requirements-only application archives; no invented distribution metadata."""
import hashlib
import csv
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import zipfile

STATE = Path('.oyzu-build')
ARCHIVE = STATE/'application.pyz'
MANIFEST = 'oyzu-application.json'
LIMIT = 512 * 1024 * 1024
EXCLUDED = {'venv', '__pycache__', 'dist', 'tests', 'test', 'node_modules', 'target'}
CONTROL_FILES = {'oyzu.toml', 'oyzu.local.toml', 'build.yaml', 'requirements.txt'}


def runtime(primary, source):
    path = Path(__file__).with_name(primary)
    if not path.exists():
        path = Path(__file__).with_name(source)
    spec = importlib.util.spec_from_file_location('oyzu_'+source.replace('.', '_'), path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def portable(name):
    devices = {'CON', 'PRN', 'AUX', 'NUL', *(f'COM{i}' for i in range(1, 10)), *(f'LPT{i}' for i in range(1, 10))}
    return bool(name) and all(
        p not in {'', '.', '..'} and not p.endswith((' ', '.'))
        and p.split('.')[0].upper() not in devices
        and not any(ord(c) < 32 or c in '<>:"\\|?*' for c in p)
        for p in name.split('/')
    )


def runtime_packages(dependencies):
    inventory = json.loads((dependencies/'packages.json').read_text())
    packages = {p['id']: p for p in inventory['packages']}
    pending = list(inventory['runtimeRoots'])
    selected = {}
    while pending:
        identity = pending.pop()
        if identity in selected:
            continue
        package = packages[identity]
        selected[identity] = package
        pending.extend(package['dependencies'])
    return [selected[key] for key in sorted(selected)]


def prepare(dependencies):
    runtime('python.py', 'adapter.py').prepare_environment(dependencies)


def build(dependencies):
    stage = STATE/'application'
    stage.mkdir(exist_ok=False)
    packages = runtime_packages(dependencies)
    wheels = []
    for package in packages:
        name = package['file']
        if not portable(name) or '/' in name or not name.endswith('.whl'):
            raise ValueError('Invalid captured application wheel')
        wheel = dependencies/'wheels'/name
        with zipfile.ZipFile(wheel) as archive:
            metadata = [p for p in archive.namelist() if p.count('/') == 1 and p.endswith('.dist-info/WHEEL')]
            if len(metadata) != 1 or 'Root-Is-Purelib: true' not in archive.read(metadata[0]).decode():
                raise ValueError('Application zip archives require pure Python runtime dependencies; native application layout integration is pending')
        wheels.append(str(wheel))
    if wheels:
        runtime('python.py', 'adapter.py').run([sys.executable, '-I', '-m', 'pip', '--isolated', 'install', '--no-index', '--no-deps', '--no-compile', '--target', str(stage), *wheels])
    # pip's local-wheel provenance contains temporary acquisition paths. Retain
    # upstream package metadata but remove that installer-only file and its row.
    for origin in stage.glob('*.dist-info/direct_url.json'):
        name = origin.relative_to(stage).as_posix()
        record = origin.with_name('RECORD')
        with record.open(newline='', encoding='utf-8') as source:
            rows = [row for row in csv.reader(source) if row[0] != name]
        origin.unlink()
        with record.open('w', newline='', encoding='utf-8') as output:
            csv.writer(output).writerows(rows)
    sources = []
    total = 0
    for directory, children, files in os.walk('.'):
        # Hidden/local configuration and build/test infrastructure are not runtime
        # payload. Visible application data remains alongside its source.
        children[:] = sorted(p for p in children if not p.startswith('.') and p not in EXCLUDED)
        for child in children:
            if (Path(directory)/child).is_symlink():
                raise ValueError('Application source directory is a symlink')
        for filename in sorted(files):
            if filename.startswith('.') or filename in CONTROL_FILES or filename.endswith(('.pyc', '.pyo')):
                continue
            path = Path(directory)/filename
            if path.is_symlink():
                raise ValueError('Application source is a symlink')
            name = path.as_posix()
            if not portable(name) or name == MANIFEST:
                raise ValueError('Invalid or reserved application source path')
            total += path.stat().st_size
            if total > LIMIT:
                raise ValueError('Application source exceeds limit')
            destination = stage/path
            if destination.exists():
                raise ValueError('Application source collides with a runtime dependency: '+name)
            destination.parent.mkdir(parents=True, exist_ok=True)
            if path.suffix == '.py':
                compile(path.read_bytes(), name, 'exec')
                sources.append(name)
            shutil.copyfile(path, destination)
    if not (stage/'__main__.py').exists():
        if not (stage/'app.py').is_file():
            raise ValueError('Application entrypoint requires app.py or __main__.py')
        (stage/'__main__.py').write_text('import runpy\nrunpy.run_module("app", run_name="__main__")\n', encoding='utf-8')
    metadata = {'kind':'python-application', 'version':os.environ['OYZU_VERSION'], 'sourceDigest':os.environ['OYZU_SOURCE_DIGEST'], 'sourceFiles':sorted(sources), 'runtimePackages':[p['id'] for p in packages]}
    (stage/MANIFEST).write_text(json.dumps(metadata, sort_keys=True)+'\n', encoding='utf-8')
    total = 0
    names = set()
    with zipfile.ZipFile(ARCHIVE, 'x', compression=zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(stage.rglob('*')):
            if path.is_symlink():
                raise ValueError('Symlink in application payload')
            if not path.is_file():
                continue
            name = path.relative_to(stage).as_posix()
            if not portable(name) or name.casefold() in names or path.suffix.lower() in {'.so', '.pyd', '.dll', '.dylib', '.exe', '.pth'}:
                raise ValueError('Unsupported zip application payload: '+name)
            names.add(name.casefold())
            total += path.stat().st_size
            if len(names) > 100000 or total > LIMIT:
                raise ValueError('Application payload exceeds limit')
            info = zipfile.ZipInfo(name, date_time=(1980,1,1,0,0,0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, path.read_bytes())


def extract(destination):
    destination.mkdir(exist_ok=False)
    names = set()
    total = 0
    with zipfile.ZipFile(ARCHIVE) as archive:
        for entry in archive.infolist():
            if not portable(entry.filename) or entry.filename.casefold() in names or entry.external_attr >> 16 & 0o170000 != 0o100000:
                raise ValueError('Invalid or duplicate application archive path')
            names.add(entry.filename.casefold())
            total += entry.file_size
            if len(names) > 100000 or total > LIMIT:
                raise ValueError('Application archive exceeds limit')
            path = destination/entry.filename
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open('xb') as output:
                output.write(archive.read(entry))
    return json.loads((destination/MANIFEST).read_text())


def test(junit, coverage):
    original = hashlib.sha256(ARCHIVE.read_bytes()).hexdigest()
    destination = (STATE/'test-application').resolve()
    metadata = extract(destination)
    if metadata['version'] != os.environ['OYZU_VERSION'] or metadata['sourceDigest'] != os.environ['OYZU_SOURCE_DIGEST']:
        raise ValueError('Application identity differs from plan')
    modules, files = set(), []
    for name in metadata['sourceFiles']:
        if not portable(name) or not name.endswith('.py'):
            raise ValueError('Invalid application source identity')
        first = name.split('/')[0]
        module = first[:-3] if '/' not in name else first
        if module.isidentifier() and module != '__main__':
            modules.add(module)
            files.append(str(destination/name))
    if not files:
        raise ValueError('No importable application source for coverage')
    sys.path.insert(0, str(destination))
    reporter = runtime('python-reporting.py', 'reporting.py')
    result = reporter.run_tests(None, Path(junit), Path(coverage), sources=(sorted(modules), sorted(files)))
    (STATE/'tested-application.json').write_text(json.dumps({'sha256':original}))
    return result


def package():
    tested = json.loads((STATE/'tested-application.json').read_text())
    if hashlib.sha256(ARCHIVE.read_bytes()).hexdigest() != tested['sha256']:
        raise ValueError('Application archive changed after testing')
    target, version = os.environ['OYZU_TARGET'], os.environ['OYZU_VERSION']
    shutil.copyfile(ARCHIVE, Path('/out')/target/'artifacts'/f'{target}-{version}.pyz')


if __name__ == '__main__':
    operation = sys.argv[1]
    if operation in {'prepare', 'build'}:
        globals()[operation](Path(sys.argv[2]) if len(sys.argv)>2 else Path('/dependencies'))
    elif operation == 'test':
        raise SystemExit(test(*sys.argv[2:]))
    elif operation == 'package':
        package()
    else:
        raise ValueError('Unknown Python application operation')
