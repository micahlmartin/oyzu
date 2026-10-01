"""Native development probe for captured Maven files and a fresh offline Gradle home.

The probe downloads from explicit Maven Central; production acquisition must use
the engine's scoped broker. This does not prove compiled-CLI sandbox behavior.
"""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import xml.etree.ElementTree as ET
from urllib.error import HTTPError
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--gradle-home', type=Path, required=True)
    parser.add_argument('--java-home', type=Path, required=True)
    args = parser.parse_args()
    base = Path(tempfile.mkdtemp(prefix='oyzu-gradle-preparation-'))
    print(f'Native probe evidence: {base}', flush=True)
    repository = base / 'repository'
    repository.mkdir()

    class Mirror(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            self.relay(True)

        def do_HEAD(self):
            self.relay(False)

        def relay(self, include_body):
            path = self.path.removeprefix('/maven/')
            if not self.path.startswith('/maven/') or any(p in {'', '.', '..'} for p in path.split('/')) or any(c in path for c in '%?\\#'):
                self.send_error(400)
                return
            destination = repository / path
            try:
                with urlopen('https://repo.maven.apache.org/maven2/' + path, timeout=60) as response:
                    body = response.read()
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(body)
                self.send_response(200)
                self.send_header('Content-Length', str(len(body)))
                self.end_headers()
                if include_body:
                    self.wfile.write(body)
            except HTTPError as error:
                self.send_error(error.code)

    server = ThreadingHTTPServer(('127.0.0.1', 0), Mirror)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    executable = args.gradle_home / 'bin' / ('gradle.bat' if os.name == 'nt' else 'gradle')
    command = [str(executable), '--no-daemon', '--no-watch-fs', '--console=plain', '--max-workers=2',
               '-Dorg.gradle.java.installations.auto-download=false',
               '-I', str(ROOT / 'src/builders/java/gradle/runtime/metadata.gradle'),
               '-I', str(ROOT / 'src/builders/java/gradle/runtime/integration.gradle')]

    def invoke(phase, goal, url, offline, success=True, change=None):
        project = base / phase
        shutil.copytree(ROOT / 'examples/builds/java-gradle-multi-project/project', project)
        if change:
            change(project)
        env = dict(os.environ, JAVA_HOME=str(args.java_home), GRADLE_USER_HOME=str(base / (phase + '-home')),
                   OYZU_GRADLE_WORKSPACE=str(project), OYZU_GRADLE_METADATA=str(base / (phase + '-metadata')),
                   OYZU_GRADLE_REPOSITORY=url, OYZU_GRADLE_SNAPSHOT='012345678901')
        result = subprocess.run(command + (['--offline'] if offline else []) + [goal],
                                cwd=project, env=env, capture_output=True, text=True, timeout=300)
        (base / (phase + '.stdout')).write_text(result.stdout)
        (base / (phase + '.stderr')).write_text(result.stderr)
        assert (result.returncode == 0) == success, result.stdout + result.stderr
        return project

    try:
        invoke('acquire', 'oyzuAcquire', f'http://127.0.0.1:{server.server_port}/maven/', False)
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
    build = invoke('build', 'oyzuBuild', repository.as_uri() + '/', True)
    models = [json.loads(p.read_text()) for p in (base / 'acquire-metadata').glob('*.json')]
    archives = [a for model in models for project in model['projects'] for a in project['archives']]
    assert len(archives) == 3 and all((build / a['file']).is_file() for a in archives)
    assert list((build / 'library/build/test-results/test').glob('TEST-*.xml'))
    assert (build / 'library/build/reports/oyzu/test/jacoco.xml').is_file()
    repeated = invoke('repeat', 'oyzuBuild', repository.as_uri() + '/', True)
    for archive in archives:
        assert hashlib.sha256((build / archive['file']).read_bytes()).digest() == hashlib.sha256((repeated / archive['file']).read_bytes()).digest()

    def break_test(project):
        source = project / 'library/src/main/java/example/Greeting.java'
        source.write_text(source.read_text().replace('Hello, Oyzu!', 'Broken greeting'))

    failed = invoke('failed', 'oyzuBuild', repository.as_uri() + '/', True, False, break_test)
    tests = list((failed / 'library/build/test-results/test').glob('TEST-*.xml'))
    assert any(ET.parse(p).getroot().findall('.//failure') for p in tests)
    assert (failed / 'library/build/reports/oyzu/test/jacoco.xml').is_file()
    print(json.dumps({'result': 'native acquisition, fresh-home offline build, repeatability and failure reports passed',
                      'repositoryFiles': sum(p.is_file() for p in repository.rglob('*')),
                      'evidence': str(base)}))


if __name__ == '__main__':
    main()
