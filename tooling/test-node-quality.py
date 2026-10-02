"""Native Node quality behavior, including default rules and unchanged check inputs."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='oyzu node quality ') as temporary:
        project = Path(temporary)
        (project/'package.json').write_text('{"name":"quality","version":"1.0.0","type":"module"}\n')
        env = {**os.environ, 'OYZU_NODE_QUALITY_HOME':str(ROOT/'tooling/images/node-quality')}
        source = project/'index.mjs'
        source.write_text('export const answer=42\n')

        def run(mode, success=True):
            before = {p:p.read_bytes() for p in project.rglob('*') if p.is_file()}
            command = [str(args.cli.resolve()),'-C',str(project),'run',mode] if args.cli else ['node',str(ROOT/'src/builders/node/runtime/quality.mjs'),mode]
            result = subprocess.run(command, cwd=project, env=env, capture_output=True, text=True, timeout=90)
            assert (result.returncode == 0) == success, (mode,result.stdout,result.stderr)
            if mode != 'format':
                assert before == {p:p.read_bytes() for p in project.rglob('*') if p.is_file()}
            return result

        run('lint')
        run('format-check', False)
        run('format')
        run('format-check')
        assert source.read_text() == 'export const answer = 42;\n'
        source.write_text('export const answer = missing;\n', newline='\n')
        assert 'no-undef' in run('lint', False).stdout
        source.write_text('export const answer = 42;\n', newline='\n')
        (project/'types.ts').write_text('export const value: number = 42;\n', newline='\n')
        run('lint')
        run('format-check')
        (project/'types.ts').write_text('export const value: any = 42;\n', newline='\n')
        assert 'no-explicit-any' in run('lint', False).stdout
        (project/'types.ts').write_text('export const value: number = 42;\n', newline='\n')
        for directory in ['node_modules','.oyzu-build','dist','coverage']:
            (project/directory).mkdir()
            (project/directory/'invalid.js').write_text('not even valid JavaScript @@@')
        run('lint')
        run('format-check')
        # Native config/ignore semantics are respected without enabling autofix.
        (project/'eslint.config.mjs').write_text('export default [{ ignores: ["ignored.mjs"] }, { rules: { "no-console": "error" } }];\n')
        (project/'ignored.mjs').write_text('not valid @@')
        (project/'.prettierignore').write_text('ignored.mjs\neslint.config.mjs\n')
        source.write_text('console.log("x");\n', newline='\n')
        assert 'no-console' in run('lint', False).stdout
        source.write_text('export const answer = 42;\n', newline='\n')
        run('lint')
        run('format-check')
        (project/'.prettierrc.json').write_text('{"semi":false}')
        run('format-check', False)
        run('format')
        assert source.read_text() == 'export const answer = 42\n'
        run('format-check')
        package = project/'package.json'
        package.write_text(json.dumps({'name':'quality','version':'1.0.0','devDependencies':{'prettier':'3.9.9'}}))
        assert 'Declared prettier is not installed' in run('format-check', False).stderr
    print('Native Node quality: default JS/TS lint, read-only formatting, explicit writes, native config/ignores, private-state exclusions and missing declared tool rejection passed')


if __name__ == '__main__':
    main()
