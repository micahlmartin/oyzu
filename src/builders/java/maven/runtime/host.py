"""Native host Maven observation and tests using provisioned tools/dependencies."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

JACOCO = 'org.jacoco:jacoco-maven-plugin:0.8.13'


def launcher(name):
    if name in {'mvn', 'mvn.cmd'}:
        home = os.environ.get('MAVEN_HOME')
        if home:
            return str(Path(home)/'bin'/('mvn.cmd' if os.name == 'nt' else 'mvn'))
        return shutil.which('mvn.cmd' if os.name == 'nt' else 'mvn') or name
    return name


def observe(native, output):
    home = os.environ.get('MAVEN_HOME')
    if not home:
        raise ValueError('Set MAVEN_HOME to the provisioned Maven distribution before direct tests')
    java_home = os.environ.get('JAVA_HOME')
    suffix = '.exe' if os.name == 'nt' else ''
    def tool(name):
        return str(Path(java_home)/'bin'/(name+suffix)) if java_home else name
    runtime = Path(__file__).parent
    with tempfile.TemporaryDirectory(prefix='oyzu-maven-model-') as directory:
        root = Path(directory)
        classes = root/'classes'
        classes.mkdir()
        subprocess.run([tool('javac'), '--release', '17', '-cp', str(Path(home)/'lib/*'),
                        '-d', str(classes), str(runtime/'OyzuMetadata.java')], check=True, stdout=sys.stderr)
        components = classes/'META-INF/plexus/components.xml'
        components.parent.mkdir(parents=True)
        shutil.copyfile(runtime/'components.xml', components)
        extension = root/'metadata.jar'
        subprocess.run([tool('jar'), '--create', '--file', str(extension), '-C', str(classes), '.'],
                       check=True, stdout=sys.stderr)
        env = dict(os.environ, OYZU_MAVEN_WORKSPACE=str(Path.cwd().resolve()),
                   OYZU_MAVEN_METADATA=str(output), OYZU_MAVEN_TEST_PLAN='true',
                   OYZU_MAVEN_TEST_PHASE='test', OYZU_MAVEN_METADATA_ONLY='true')
        subprocess.run([launcher(native), '-B', '-o', '-ntp', '-Dmaven.ext.class.path='+str(extension),
                        'validate'], env=env, check=True, stdout=sys.stderr)


def test(native, encoded, output, extra):
    if extra:
        raise ValueError('Maven direct reporting does not yet support extra task arguments')
    spec = importlib.util.spec_from_file_location('oyzu_maven_reporting', Path(__file__).with_name('maven_reporting.py'))
    reporting = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(reporting)
    modules = json.loads(encoded)
    reporting.clear([module['reports'] for module in modules])
    # Avoid stale coverage on both successful and failed reruns. Maven still
    # owns compilation outputs and its conventional per-module coverage paths.
    for module in modules:
        directory = reporting.directory(module['directory'])
        for path in [directory/'jacoco.exec', directory/'site/jacoco/jacoco.xml']:
            path.unlink(missing_ok=True)
    command = [launcher(native), '-B', '-o', '-ntp']
    status = subprocess.run([*command, JACOCO+':prepare-agent', 'test', JACOCO+':report']).returncode
    if status:
        subprocess.run([*command, JACOCO+':report'])
    for module in modules:
        destination = Path(output)/module['id']
        destination.mkdir(parents=True, exist_ok=True)
        reporting.combined(module['reports'], destination/'junit.xml')
        source = reporting.directory(module['directory'])/'site/jacoco/jacoco.xml'
        if source.is_file():
            shutil.copyfile(source, destination/'jacoco.xml')
    return status


if __name__ == '__main__':
    mode, native, *args = sys.argv[1:]
    if mode == 'describe':
        observe(native, Path(args[0]))
    elif mode == 'test':
        raise SystemExit(test(native, args[0], args[1], args[2:]))
    else:
        raise ValueError('Unknown Maven host operation')
