"""Native Java quality tasks through the compiled CLI, without Maven/Gradle/Ant execution."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--tools', type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    env = {**os.environ, 'OYZU_JAVA_QUALITY_HOME':str(args.tools.resolve())}
    for manager, manifest, body in [
        ('maven', 'pom.xml', '<project><modelVersion>4.0.0</modelVersion><groupId>example</groupId><artifactId>app</artifactId><version>1.0</version></project>'),
        ('gradle', 'build.gradle', "plugins { id 'java' }\n"),
        ('ant', 'build.xml', '<project><target name="compile"/></project>'),
    ]:
        with tempfile.TemporaryDirectory(prefix='oyzu Java quality ') as temporary:
            project = Path(temporary)
            (project/manifest).write_text(body)
            source = project/'custom source root'/'App.java'
            source.parent.mkdir()
            good = 'class App {\n  int answer() {\n    return 42;\n  }\n}\n'
            source.write_text(good, newline='\n')

            def run(*argv, success=True, environment=env):
                result = subprocess.run([str(cli), '-C', str(project), *argv], env=environment,
                                        capture_output=True, text=True, encoding='utf-8', timeout=90)
                assert (result.returncode == 0) == success, (manager, argv, result.stdout, result.stderr)
                return result

            listed = json.loads(run('run', 'list', '--json', environment={**env, 'PATH':''}).stdout)
            for name in ['lint', 'format-check', 'format']:
                assert listed['project:'+name]['availability'] is None
            assert listed['project:lint']['build_stage'] and listed['project:format-check']['build_stage']
            assert listed['project:format']['mutates_source'] and not listed['project:format']['build_stage']
            before = source.read_bytes()
            run('run', 'lint')
            run('run', 'format-check')
            assert source.read_bytes() == before
            missing = {k:v for k,v in env.items() if k != 'OYZU_JAVA_QUALITY_HOME'}
            failure = run('run', 'lint', success=False, environment=missing)
            assert 'OYZU_JAVA_QUALITY_HOME' in failure.stderr
            # Native linter must detect a semantic violation without changing files.
            source.write_text('import java.util.List;\n' + good, newline='\n')
            before = source.read_bytes()
            failure = run('run', 'lint', success=False)
            assert 'UnusedImports' in failure.stdout + failure.stderr
            assert source.read_bytes() == before
            source.write_text('class App{int answer(){return 42;}}\n', newline='\n')
            before = source.read_bytes()
            run('run', 'format-check', success=False)
            run('run', 'format-check', '--replace', success=False)
            assert source.read_bytes() == before
            run('run', 'format')
            run('run', 'format-check')
            assert source.read_text() == good
            # Native output trees are excluded, custom source roots are included.
            for name in ['build', 'target', 'dist']:
                (project/name).mkdir()
                (project/name/'Invalid.java').write_text('invalid Java')
            run('run', 'lint')
            run('run', 'format-check')
            source.write_text('class App { BROKEN }\n')
            run('run', 'lint', success=False)
            run('run', 'format-check', success=False)
            source.write_text(good, newline='\n')
            if manager == 'maven':
                # Multiple native batches: an early failure must not be hidden by
                # later successful batches. Paths with spaces remain single arguments.
                batch_root = project/('many source files ' + 'x' * 100)
                batch_root.mkdir()
                for index in range(90):
                    candidate = batch_root/f'Sample{index:03}.java'
                    prefix = 'import java.util.List;\n' if index == 0 else ''
                    candidate.write_text(prefix + f'class Sample{index:03} {{}}\n', newline='\n')
                failed = run('run', 'lint', success=False)
                assert 'UnusedImports' in failed.stdout + failed.stderr
                (batch_root/'Sample000.java').write_text('class Sample000 {}\n', newline='\n')
                run('run', 'lint')
                run('run', 'format-check')
                link = project/'linked-source.java'
                try:
                    link.symlink_to(source)
                except OSError:
                    print('File symlink probe unavailable on this host')
                else:
                    try:
                        failed = run('run', 'lint', success=False)
                        assert 'symbolic links' in failed.stdout + failed.stderr
                    finally:
                        link.unlink()
            # Explicit replacements keep their own command body.
            (project/'oyzu.toml').write_text('[tasks.lint]\nargv=["java","-version"]\n')
            run('run', 'lint', environment=missing)
            if manager == 'ant':
                (project/'oyzu.toml').unlink()
                (project/manifest).write_text('<project><target name="lint"/><target name="format-check"/><target name="format"/></project>')
                native = json.loads(run('run', 'list', '--json').stdout)
                assert native['project:lint']['argv'] == ['ant','lint']
                assert native['project:format-check']['argv'] == ['ant','format-check']
                assert native['project:format']['mutates_source']
            print(f'{manager}: native lint, read-only format gate, explicit format, overrides and source boundaries passed')


if __name__ == '__main__':
    main()
