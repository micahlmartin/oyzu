"""Native Gradle composite test execution without acquisition or version changes."""
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys

RUNTIME = Path(__file__).parent


def command(native):
    if native == 'gradle':
        home = os.environ.get('GRADLE_HOME')
        native = str(Path(home)/'bin'/('gradle.bat' if os.name == 'nt' else 'gradle')) if home else (
            shutil.which('gradle.bat' if os.name == 'nt' else 'gradle') or native)
    return [native, '--no-daemon', '--no-watch-fs', '--no-build-cache', '--no-configuration-cache',
            '--console=plain', '--max-workers=2', '--offline',
            '-Dorg.gradle.java.installations.auto-download=false',
            '-I', str(RUNTIME/'reporting.gradle'), '-I', str(RUNTIME/'host.gradle')]


def describe(native, output):
    env = dict(os.environ, OYZU_GRADLE_WORKSPACE=str(Path.cwd().resolve()),
               OYZU_GRADLE_METADATA=output, OYZU_GRADLE_DESCRIBE_ONLY='true')
    return subprocess.run([*command(native), '-I', str(RUNTIME/'metadata.gradle'), 'oyzuCollectMetadata'],
                          env=env, stdout=sys.stderr).returncode


def contained(value):
    root = Path.cwd().resolve()
    path = root/value
    if not path.resolve().is_relative_to(root) or path.is_symlink():
        raise ValueError('Gradle native report escapes the project')
    return path


def documents(paths):
    remaining = 64*1024*1024
    for path in paths:
        if path.is_symlink() or not path.is_file():
            raise ValueError('Expected a native Gradle JUnit file')
        with path.open('rb') as stream:
            data = stream.read(min(16*1024*1024, remaining)+1)
        if len(data) > min(16*1024*1024, remaining):
            raise ValueError('Gradle native test reports exceed collection budget')
        remaining -= len(data)
        yield data


def test(native, encoded, output, extra):
    if extra:
        raise ValueError('Gradle direct reporting does not yet support extra task arguments')
    modules = json.loads(encoded)
    for module in modules:
        for path in contained(module['junit']).glob('TEST-*.xml'):
            path.unlink()
        contained(module['coverage']).unlink(missing_ok=True)
    # Native rerun semantics ensure that a previous Gradle UP-TO-DATE result
    # cannot satisfy this invocation's report obligations after output cleanup.
    env = dict(os.environ, OYZU_GRADLE_WORKSPACE=str(Path.cwd().resolve()), OYZU_GRADLE_TEST_PLAN=encoded)
    status = subprocess.run([*command(native), '--continue', '--rerun-tasks', 'oyzuHostTest'], env=env).returncode
    combine = runpy.run_path(str(RUNTIME/'java-junit.py'))['combine']
    for module in modules:
        destination = Path(output)/module['id']
        destination.mkdir(parents=True, exist_ok=True)
        combine(documents(sorted(contained(module['junit']).glob('TEST-*.xml'))), destination/'junit.xml')
        coverage = contained(module['coverage'])
        if coverage.is_file():
            shutil.copyfile(coverage, destination/'jacoco.xml')
    return status


if __name__ == '__main__':
    mode, native, *args = sys.argv[1:]
    if mode == 'describe':
        raise SystemExit(describe(native, args[0]))
    if mode == 'test':
        raise SystemExit(test(native, args[0], args[1], args[2:]))
    raise ValueError('Unknown Gradle host operation')
