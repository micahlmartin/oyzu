"""Native multi-project/composite tests through the compiled CLI and report bundle."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--gradle-home', type=Path, required=True)
    parser.add_argument('--user-home', type=Path, required=True)
    parser.add_argument('--acquire', action='store_true', help='Explicit online native fixture setup before product tests')
    args = parser.parse_args()
    cli = args.cli.resolve()
    env = dict(os.environ, GRADLE_HOME=str(args.gradle_home.resolve()), GRADLE_USER_HOME=str(args.user_home.resolve()))
    native = args.gradle_home.resolve()/('bin/gradle.bat' if os.name == 'nt' else 'bin/gradle')
    spec = importlib.util.spec_from_file_location('build_scenarios_runner', ROOT/'tooling/test-build-scenarios.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    with tempfile.TemporaryDirectory(prefix='oyzu Gradle direct ', ignore_cleanup_errors=True) as directory:
        for case in ['authored', 'included-test']:
            project = Path(directory)/case
            shutil.copytree(ROOT/'examples/builds/java-gradle-multi-project/project', project)
            count = 1
            if case == 'included-test':
                count = 2
                with (project/'support/build.gradle').open('a', encoding='utf-8') as stream:
                    stream.write('\nrepositories { mavenCentral() }\ndependencies { testImplementation "org.junit.jupiter:junit-jupiter:5.11.4"; testRuntimeOnly "org.junit.platform:junit-platform-launcher:1.11.4" }\ntest { useJUnitPlatform() }\n')
                source = project/'support/src/test/java/example/SupportTest.java'
                source.parent.mkdir(parents=True)
                source.write_text('package example; import org.junit.jupiter.api.Test; import static org.junit.jupiter.api.Assertions.assertEquals; class SupportTest { @Test void actual() { assertEquals("support", Support.marker()); } }\n', encoding='utf-8')
                with (project/'library/build.gradle').open('a', encoding='utf-8') as stream:
                    stream.write('\ntest { reports.junitXml.outputLocation = layout.buildDirectory.dir("custom test reports") }\n')
            originals = {p: p.read_bytes() for p in project.rglob('*') if p.is_file()}
            if args.acquire:
                runtime = ROOT/'src/builders/java/gradle/runtime'
                subprocess.run([str(native), '--no-daemon', '--console=plain', '--max-workers=2',
                                '-Dorg.gradle.java.installations.auto-download=false',
                                '-I', str(runtime/'reporting.gradle'), ':library:test',
                                *([':support:test'] if case == 'included-test' else [])],
                               env=env, cwd=project, check=True, timeout=600)

            def invoke(*argv, success=True):
                result = subprocess.run([str(cli), '-C', str(project), *argv], env=env,
                                        capture_output=True, text=True, encoding='utf-8', timeout=300)
                assert (result.returncode == 0) == success, (case, argv, result.stdout, result.stderr)
                return result

            def bundle(success=True):
                manifest = harness.validate(project/'dist')
                assert manifest['status'] == ('succeeded' if success else 'failed')
                assert not manifest['artifacts'] and len(manifest['reports']) == count*2
                tests = [r for r in manifest['reports'] if r['kind'] == 'test']
                assert sum(r['summary']['failed'] for r in tests) == (0 if success else 1)
                assert sum(r['summary']['passed'] for r in tests) == count-(0 if success else 1)
                assert all(r['summary']['covered'] > 0 for r in manifest['reports'] if r['kind'] == 'coverage')
                invoke('inspect', 'dist')
                return manifest

            tasks = json.loads(invoke('run', 'list', '--json').stdout)
            assert tasks['project:test']['argv'] == ['gradle', '--no-daemon', 'test']
            invoke('run', 'test')
            bundle()
            assert all(p.read_bytes() == data for p, data in originals.items())
            (project/'oyzu.toml').write_text('[tasks."project:post_test"]\nargv=["python","-c","from pathlib import Path; Path(\'hook-ran\').write_text(\'yes\')"]\n', encoding='utf-8')
            invoke('run', 'project:test')
            assert bundle()['actions'][-1]['id'] == 'project:post_test'
            assert (project/'hook-ran').is_file()
            (project/'hook-ran').unlink()
            source = project/('library/src/test/java/example/GreetingTest.java' if case == 'authored'
                              else 'support/src/test/java/example/SupportTest.java')
            old = 'Hello, Oyzu!' if case == 'authored' else '"support"'
            source.write_text(source.read_text(encoding='utf-8').replace(old, 'wrong' if case == 'authored' else '"wrong"'), encoding='utf-8')
            invoke('run', 'test', success=False)
            assert bundle(False)['actions'][-1]['status'] == 'blocked'
            assert not (project/'hook-ran').exists()
            print(f'Gradle {case}: native tests, per-task JUnit/JaCoCo, reruns, hooks, failure evidence, unchanged sources and inspected bundles passed', flush=True)


if __name__ == '__main__':
    main()
