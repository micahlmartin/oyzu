"""Native Maven preparation and offline lifecycle; routing stays scoped to Central.

The host broker alone contacts upstream. Maven sees a loopback mirror in its
network-disabled container, then an immutable acquired repository during builds.
"""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import threading
import xml.etree.ElementTree as ET

VERSIONS = 'org.codehaus.mojo:versions-maven-plugin:2.19.1:set'
DEPENDENCY = 'org.apache.maven.plugins:maven-dependency-plugin:3.8.1'
JACOCO = 'org.jacoco:jacoco-maven-plugin:0.8.13'
CENTRAL = 'https://repo.maven.apache.org/maven2/'
EXTENSION = '/opt/oyzu-maven/metadata.jar'


def transport():
    spec = importlib.util.spec_from_file_location('oyzu_broker_transport', Path(__file__).with_name('broker_transport.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class Mirror(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        self.relay(True)

    def do_HEAD(self):
        self.relay(False)

    def relay(self, include_body):
        if not self.path.startswith('/maven/') or '?' in self.path or '#' in self.path:
            self.send_error(403)
            return
        try:
            info, body = transport().fetch(CENTRAL + self.path[len('/maven/'):])
        except TimeoutError:
            self.send_error(504)
            return
        self.send_response(info['status'])
        self.send_header('Content-Type', info['contentType'])
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        if include_body:
            self.wfile.write(body)


def settings(path, url):
    path.write_text(f'''<settings xmlns="http://maven.apache.org/SETTINGS/1.2.0">
<mirrors><mirror><id>oyzu-central</id><mirrorOf>*</mirrorOf><url>{url}</url></mirror></mirrors>
<interactiveMode>false</interactiveMode></settings>''')


def maven(state, goals, offline=False, check=True):
    env = dict(os.environ, MAVEN_SKIP_RC='true', OYZU_MAVEN_WORKSPACE=str(Path.cwd()),
               OYZU_MAVEN_METADATA=str(state / 'metadata.xml'))
    env.pop('MAVEN_ARGS', None)
    env.pop('MAVEN_OPTS', None)
    command = ['mvn', '-B', '-ntp', '-C', '-Dstyle.color=never', '-s', str(state / 'settings.xml'), '-gs', str(state / 'settings.xml'),
               '-Dmaven.repo.local=' + str(state / 'repository'), '-Duser.home=/tmp/oyzu-home',
               '-Dmaven.ext.class.path=' + EXTENSION, '-Dproject.build.outputTimestamp=315532800']
    if offline:
        command.append('-o')
    result = subprocess.run(command + goals, env=env)
    if check and result.returncode:
        raise RuntimeError('native Maven operation failed: ' + ' '.join(goals))
    return result.returncode


def projects(state):
    return ET.parse(state / 'metadata.xml').findall('project')


def contained(path):
    value = Path(path)
    if value.is_absolute() or '..' in value.parts or '\\' in path:
        raise ValueError('native Maven path escapes captured project')
    resolved = value.resolve()
    if not resolved.is_relative_to(Path.cwd()):
        raise ValueError('native Maven path escapes captured project')
    return value


def acquire(digest):
    state = Path('/out')
    server = ThreadingHTTPServer(('127.0.0.1', 0), Mirror)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        settings(state / 'settings.xml', f'http://127.0.0.1:{server.server_port}/maven/')
        maven(state, ['validate'])
        original = projects(state)
        shutil.copyfile(state / 'metadata.xml', state / 'original.xml')
        versions = dict.fromkeys(p.findtext('version') for p in original)
        projected = {}
        for version in versions:
            if not version or not re.fullmatch(r'[A-Za-z0-9_.-]+', version):
                raise ValueError('unresolved or unsafe Maven version')
            new = version.removesuffix('-SNAPSHOT') + '-dev.g' + digest[7:19]
            projected[version] = new
            maven(state, [VERSIONS, '-DgroupId=*', '-DartifactId=*', '-DoldVersion=' + version,
                          '-DnewVersion=' + new, '-DprocessAllModules=true', '-DgenerateBackupPoms=false'])
        maven(state, ['validate'])
        final = projects(state)
        expected = {(p.findtext('groupId'), p.findtext('artifactId')): projected[p.findtext('version')] for p in original}
        actual = {(p.findtext('groupId'), p.findtext('artifactId')): p.findtext('version') for p in final}
        if actual != expected:
            raise ValueError('native Maven version projection did not preserve reactor coordinates')
        # Capture only native POM changes; never replay preparation side effects.
        for project in final:
            pom = contained(project.findtext('pom'))
            destination = state / 'overlay' / pom
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(pom, destination)
        maven(state, [DEPENDENCY + ':go-offline'])

        def get(artifact):
            maven(state, ['-N', DEPENDENCY + ':get', '-Dartifact=' + artifact, '-Dtransitive=true'])
        # These are pinned reporting integrations, acquired before offline work.
        get(JACOCO)
        get('org.jacoco:org.jacoco.agent:0.8.13:jar:runtime')
        providers = set()
        for project in final:
            junit = any(d.findtext('groupId', '').startswith('org.junit.jupiter') for d in project.findall('dependencies/dependency'))
            if junit:
                for plugin in project.findall('plugins/plugin'):
                    if plugin.findtext('artifactId') in {'maven-surefire-plugin', 'maven-failsafe-plugin'}:
                        version = plugin.findtext('version')
                        if not version:
                            raise ValueError('native test provider has no resolved plugin version')
                        providers.add('org.apache.maven.surefire:surefire-junit-platform:' + version)
        for provider in sorted(providers):
            get(provider)
        engine = state / 'repository/org/junit/platform/junit-platform-engine'
        if engine.exists():
            for version in sorted(p.name for p in engine.iterdir() if p.is_dir() and list(p.glob('*.jar'))):
                get('org.junit.platform:junit-platform-launcher:' + version)
        # Metadata queries after acquisition restore the entire native reactor;
        # dependency:get's -N metadata is intentionally not the planned graph.
        maven(state, ['validate'])
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
    settings(state / 'settings.xml', 'http://127.0.0.1:9/unavailable/')
    for path in (state / 'repository').rglob('*'):
        if path.is_file() and (path.name.endswith('.lastUpdated') or path.name == 'resolver-status.properties'):
            path.unlink()
        elif path.name == '_remote.repositories':
            path.write_text('\n'.join(sorted(line for line in path.read_text().splitlines() if line and not line.startswith('#'))) + '\n')
    (state / 'manager-version.txt').write_text(subprocess.check_output(['mvn', '-Dstyle.color=never', '--version'], text=True).splitlines()[0] + '\n')


def install():
    state = Path.cwd() / '.oyzu-maven'
    state.mkdir()
    shutil.copytree('/dependencies/repository', state / 'repository')
    shutil.copyfile('/dependencies/settings.xml', state / 'settings.xml')
    for pom in Path('/dependencies/overlay').rglob('*'):
        if not pom.is_file():
            continue
        relative = pom.relative_to('/dependencies/overlay')
        destination = contained(relative.as_posix())
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(pom, destination)


def build():
    state = Path.cwd() / '.oyzu-maven'
    # One native reactor lifecycle owns compilation, unit/integration tests and
    # configured checks. Never separately run test after Maven verify.
    status = maven(state, [JACOCO + ':prepare-agent', 'verify', JACOCO + ':report'], offline=True, check=False)
    if status:
        # A failed lifecycle must not erase already recorded coverage. The
        # report goal does not compile or re-run tests; preserve the first code.
        maven(state, [JACOCO + ':report'], offline=True, check=False)
    return status


def package(manifest):
    for artifact in json.loads(manifest):
        source = contained(artifact['source'])
        if not source.is_file() or source.is_symlink():
            raise ValueError('missing native Maven artifact: ' + str(source))
        destination = Path(artifact['destination'])
        with source.open('rb') as incoming, destination.open('xb') as outgoing:
            shutil.copyfileobj(incoming, outgoing)


if __name__ == '__main__':
    operation = sys.argv[1]
    if operation == 'acquire':
        acquire(sys.argv[2])
    elif operation == 'install':
        install()
    elif operation == 'build':
        sys.exit(build())
    elif operation == 'package':
        package(sys.argv[2])
    else:
        raise ValueError('unknown native Maven operation')
