"""Real terminal acceptance: compiled CLI, two captured Rust builds and failures.

Install tooling/terminal-requirements.txt; provision the documented Rust image.
Windows uses ConPTY via pywinpty; Unix uses the standard-library pty module.
"""
import argparse
import codecs
import json
import os
from pathlib import Path
import queue
import shutil
import subprocess
import threading
import time

import pyte

parser = argparse.ArgumentParser()
parser.add_argument('--cli', required=True, type=Path)
parser.add_argument('--evidence-dir', required=True, type=Path)
args = parser.parse_args()
cli = args.cli.resolve()
evidence = args.evidence_dir.resolve()
evidence.mkdir(parents=True, exist_ok=True)
project = evidence / 'project'
project.mkdir()
root = Path(__file__).resolve().parents[1]
for target in ['alpha', 'beta']:
    shutil.copytree(root / 'examples/builds/rust-app/project', project / target)
(project/'build.yaml').write_text('alpha: {uses: rust/app, path: alpha}\nbeta: {uses: rust/app, path: beta}\n', encoding='utf-8', newline='\n')
config = '[build]\njobs=2\n'
for target in ['alpha', 'beta']:
    argv = ['python3', '-c', f'import sys,time; print("{target} live stdout",flush=True); print("{target} live stderr",file=sys.stderr,flush=True); time.sleep(6)']
    config += f'[tasks."{target}:pre_build"]\nargv={json.dumps(argv)}\n'
(project/'oyzu.toml').write_text(config, encoding='utf-8', newline='\n')


class Terminal:
    def __init__(self, command, env):
        if os.name == 'nt':
            from winpty import PtyProcess
            self.process = PtyProcess.spawn(command, env=env, dimensions=(34, 110))
        else:
            import pty
            self.fd, slave = pty.openpty()
            self.process = subprocess.Popen(command, env=env, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            self.resize(34, 110)

    def read(self):
        if os.name == 'nt':
            return self.process.read(65536)
        return os.read(self.fd, 65536)

    def write(self, value):
        if os.name == 'nt':
            self.process.write(value)
        else:
            os.write(self.fd, value.encode())

    def resize(self, rows, columns):
        if os.name == 'nt':
            self.process.setwinsize(rows, columns)
        else:
            import fcntl
            import struct
            import termios
            fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack('HHHH', rows, columns, 0, 0))

    def code(self):
        return self.process.exitstatus if os.name == 'nt' else self.process.poll()

    def close(self):
        if os.name == 'nt':
            self.process.close(force=True)
        else:
            if self.process.poll() is None:
                self.process.kill()
                self.process.wait()
            os.close(self.fd)


def run(name, failure=False):
    env = os.environ.copy()
    for key in ['CI', 'GITHUB_ACTIONS', 'GITLAB_CI', 'TF_BUILD', 'BUILDKITE', 'JENKINS_URL', 'TEAMCITY_VERSION', 'GITHUB_STEP_SUMMARY']:
        env.pop(key, None)
    env['TERM'] = 'xterm-256color'
    command = [str(cli), '--root', str(project), 'build']
    (evidence/f'{name}.command.json').write_text(json.dumps(command), encoding='utf-8')
    terminal = Terminal(command, env)
    chunks = queue.Queue()
    def read():
        decoder = codecs.getincrementaldecoder('utf-8')('replace')
        try:
            with (evidence/f'{name}.raw.txt').open('w', encoding='utf-8') as raw:
                while True:
                    chunk = terminal.read()
                    if not chunk:
                        break
                    text = decoder.decode(chunk) if isinstance(chunk, bytes) else chunk
                    raw.write(text)
                    raw.flush()
                    chunks.put(text)
        except (EOFError, OSError):
            pass
        finally:
            chunks.put(None)
    reader = threading.Thread(target=read, daemon=True)
    reader.start()
    screen = pyte.Screen(110, 34)
    stream = pyte.Stream(screen)
    started = time.monotonic()
    stage = 0
    seen = set()
    last_action = 0
    last_frame = 0
    transcript = []
    try:
        while time.monotonic() - started < 600:
            try:
                chunk = chunks.get(timeout=1)
            except queue.Empty:
                if terminal.code() is not None:
                    break
                continue
            if chunk is None:
                break
            transcript.append(chunk)
            stream.feed(chunk)
            now = time.monotonic()
            if now - last_frame < 0.1:
                continue
            last_frame = now
            frame = '\n'.join(screen.display)
            if 'OYZU' in frame and 'TARGET' in frame:
                seen.add('overview')
            if failure:
                if 'stderr | deliberate terminal failure' in frame and 'alpha / alpha:pre_test' in frame and 'failure-focus' not in seen:
                    seen.add('failure-focus')
                    (evidence/'failure-focus.txt').write_text(frame, encoding='utf-8')
                    terminal.write('q')
                continue
            if stage == 0 and 'stdout | alpha live stdout' in frame and 'stdout | beta live stdout' in frame:
                assert '2 active' in frame, frame
                (evidence/'parallel.txt').write_text(frame, encoding='utf-8')
                seen.add('parallel')
                terminal.write('\x1b[B\r')
                stage = 1
                last_action = now
            elif stage == 1 and 'alpha / alpha:pre_build' in frame:
                seen.add('selection')
                terminal.write('l')
                stage = 2
            elif stage == 2 and 'All logs' in frame:
                seen.add('all-logs')
                terminal.write('\x1b[5~')
                terminal.resize(28, 90)
                screen.resize(28, 90)
                stage = 3
                last_action = now
            elif stage == 3 and now - last_action > 0.4:
                (evidence/'resized.txt').write_text(frame, encoding='utf-8')
                seen.add('resize-scroll')
                terminal.resize(34, 110)
                screen.resize(34, 110)
                terminal.write('fa')
                stage = 4
        else:
            raise AssertionError('terminal build timed out')
        reader.join(timeout=5)
        text = ''.join(transcript)
        (evidence/f'{name}.terminal.txt').write_text(text, encoding='utf-8')
        assert terminal.code() == (1 if failure else 0), (terminal.code(), text[-4000:])
        assert 'Oyzu build: ' + ('failed' if failure else 'succeeded') in text
        assert 'overview' in seen
        if failure:
            assert 'failure-focus' in seen, seen
            assert 'Terminal view closed; build continues' in text
        else:
            assert {'parallel', 'selection', 'all-logs', 'resize-scroll'} <= seen, seen
        manifest = json.loads((project/'dist/manifest.json').read_text(encoding='utf-8'))
        assert manifest['status'] == ('failed' if failure else 'succeeded')
        if not failure:
            assert len(manifest['artifacts']) == 4
        else:
            assert any(a['id']=='alpha:package' and a['status']=='blocked' for a in manifest['actions'])
        shutil.copytree(project/'dist', evidence/f'{name}-bundle')
        subprocess.run([str(cli), '--root', str(project), 'inspect', 'dist'], check=True, capture_output=True)
    finally:
        terminal.close()


run('success')
config += '[tasks."alpha:pre_test"]\nargv=["python3","-c","import sys; print(\'deliberate terminal failure\',file=sys.stderr); sys.exit(7)"]\n'
config += '[tasks."beta:pre_test"]\nargv=["python3","-c","import time; time.sleep(4)"]\n'
(project/'oyzu.toml').write_text(config, encoding='utf-8', newline='\n')
run('failure', failure=True)
print(f'Terminal parallel panels, navigation, resize, failure focus, hide, receipts and bundles passed: {evidence}')
