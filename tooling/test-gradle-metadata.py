"""Exercise Gradle's native composite model without resolving project dependencies.

This is an adapter check, not compiled-CLI build acceptance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]


def files(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file() and '.gradle' not in p.parts}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--gradle-home', type=Path, required=True)
    parser.add_argument('--java-home', type=Path)
    args = parser.parse_args()
    executable = args.gradle_home / 'bin' / ('gradle.bat' if os.name == 'nt' else 'gradle')
    # A single-use Gradle daemon may briefly retain Windows native-library locks.
    with tempfile.TemporaryDirectory(prefix='oyzu-gradle-metadata-', ignore_cleanup_errors=True) as temporary:
        base = Path(temporary)
        project = base / 'project'
        shutil.copytree(ROOT / 'examples/builds/java-gradle-multi-project/project', project)
        before = files(project)
        output = base / 'metadata'
        env = dict(os.environ, GRADLE_USER_HOME=str(base / 'home'),
                   OYZU_GRADLE_WORKSPACE=str(project), OYZU_GRADLE_METADATA=str(output))
        if args.java_home:
            env['JAVA_HOME'] = str(args.java_home)
        command = [str(executable), '--no-daemon', '--no-watch-fs', '--offline', '--console=plain', '--max-workers=2',
                   '-Dorg.gradle.java.installations.auto-download=false',
                   '-I', str(ROOT / 'src/builders/java/gradle/runtime/metadata.gradle'), 'oyzuCollectMetadata']

        def invoke(success=True):
            result = subprocess.run(command, cwd=project, env=env, capture_output=True, text=True, timeout=180)
            assert (result.returncode == 0) == success, result.stdout + result.stderr
            return result.stdout + result.stderr

        invoke()
        documents = {d['directory']: d for d in (json.loads(p.read_text()) for p in output.glob('*.json'))}
        assert set(documents) == {'.', 'support'}
        primary = documents['.']
        assert primary['included'] == [{'name': 'support', 'path': 'support'}]
        projects = {p['path']: p for p in primary['projects']}
        assert set(projects) == {':', ':app', ':library'}
        assert projects[':library']['archives'][0]['file'] == 'library/build/libs/library.jar'
        assert projects[':library']['tests'][0]['junit'] == 'library/build/test-results/test'
        assert 'build' in projects[':app']['tasks']
        support = documents['support']['projects'][0]
        assert support['group'] == 'example.oyzu' and support['version'] == '0.1.0'
        assert support['archives'][0]['file'] == 'support/build/libs/support-0.1.0.jar'
        assert files(project) == before, 'metadata changed native project sources'
        # Native providers and custom paths must be reflected rather than guessed.
        build = project / 'library/build.gradle'
        build.write_text(build.read_text() + '\nversion = "2.3.4"\ntasks.named("jar") { archiveBaseName = "custom"; archiveClassifier = "api" }\n')
        invoke()
        updated = json.loads((output / '2e.json').read_text())
        library = next(p for p in updated['projects'] if p['path'] == ':library')
        assert library['archives'][0]['file'] == 'library/build/libs/custom-2.3.4-api.jar'
        build.write_text(build.read_text() + '\nlayout.buildDirectory = file("../../escape")\n')
        assert 'outside captured source' in invoke(False)
    print('Native Gradle composite metadata, task/archive providers and containment passed; build acceptance remains separate.')


if __name__ == '__main__':
    main()
