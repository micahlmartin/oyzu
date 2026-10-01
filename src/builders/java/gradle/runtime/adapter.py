"""Native Gradle acquisition and offline execution through captured repository files."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading

CENTRAL = 'https://repo.maven.apache.org/maven2/'
RUNTIME = Path(__file__).parent


def contained(root, relative):
    if not relative or any(c in relative for c in '\\:%?#'):
        raise ValueError('invalid native Gradle path')
    path = Path(relative)
    if path.is_absolute() or '..' in path.parts:
        raise ValueError('Gradle path escapes captured root')
    result = root / path
    if not result.resolve().is_relative_to(root.resolve()):
        raise ValueError('Gradle path escapes captured root')
    return result


def gradle(goal, repository, digest, metadata, offline):
    env = dict(os.environ, GRADLE_USER_HOME='/tmp/oyzu-gradle-home',
               OYZU_GRADLE_WORKSPACE=str(Path.cwd()), OYZU_GRADLE_METADATA=str(metadata),
               OYZU_GRADLE_REPOSITORY=repository, OYZU_GRADLE_SNAPSHOT=digest[7:19])
    env.pop('GRADLE_OPTS', None)
    env.pop('JAVA_OPTS', None)
    command = ['gradle', '--no-daemon', '--no-watch-fs', '--no-build-cache', '--no-configuration-cache',
               '--console=plain', '--max-workers=2', '-Dorg.gradle.java.installations.auto-download=false',
               '-I', str(RUNTIME / 'metadata.gradle'), '-I', str(RUNTIME / 'integration.gradle')]
    if offline:
        command.append('--offline')
    return subprocess.run(command + [goal], env=env).returncode


def acquire(digest):
    state = Path('/out')
    repository = state / 'repository'
    repository.mkdir()
    spec = importlib.util.spec_from_file_location('oyzu_broker_transport', RUNTIME / 'broker_transport.py')
    transport = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(transport)

    class Mirror(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            self.relay(True)

        def do_HEAD(self):
            self.relay(False)

        def relay(self, include_body):
            if not self.path.startswith('/maven/'):
                self.send_error(403)
                return
            path = self.path[len('/maven/'):]
            try:
                destination = contained(repository, path)
                info, body = transport.fetch(CENTRAL + path)
            except (TimeoutError, ValueError):
                self.send_error(502)
                return
            if info['status'] == 200:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(body)
            self.send_response(info['status'])
            self.send_header('Content-Type', info['contentType'])
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            if include_body:
                self.wfile.write(body)

    server = ThreadingHTTPServer(('127.0.0.1', 0), Mirror)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        status = gradle('oyzuAcquire', f'http://127.0.0.1:{server.server_port}/maven/', digest, state / 'metadata', False)
        if status:
            raise RuntimeError('native Gradle acquisition failed')
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def package(manifest):
    for artifact in json.loads(manifest):
        source = contained(Path.cwd(), artifact['source'])
        if not source.is_file() or source.is_symlink():
            raise ValueError('missing native Gradle artifact')
        with source.open('rb') as incoming, Path(artifact['destination']).open('xb') as outgoing:
            shutil.copyfileobj(incoming, outgoing)


if __name__ == '__main__':
    operation = sys.argv[1]
    if operation == 'acquire':
        acquire(sys.argv[2])
    elif operation == 'build':
        sys.exit(gradle('oyzuBuild', 'file:///dependencies/repository/', sys.argv[2], Path('/tmp/oyzu-gradle-metadata'), True))
    elif operation == 'package':
        package(sys.argv[2])
    else:
        raise ValueError('unknown native Gradle operation')
