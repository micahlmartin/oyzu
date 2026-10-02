"""Native Biome checks through the owned runtime or compiled CLI."""
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
    with tempfile.TemporaryDirectory(prefix='oyzu biome quality ') as temporary:
        project = Path(temporary)
        manifest = project/'package.json'
        manifest.write_text('{"name":"quality","version":"1.0.0","type":"module"}\n')
        config = {'files':{'includes':['**','!ignored.js']},'linter':{'rules':{'recommended':False,'suspicious':{'noDebugger':'error'}}},'formatter':{'indentStyle':'space','indentWidth':2}}
        (project/'biome.jsonc').write_text('// Native JSONC configuration\n'+json.dumps(config)+'\n')
        source = project/'index.js'
        source.write_text('export const value=42;\n',newline='\n')
        data = project/'data.json'
        data.write_text('{"value":42}\n',newline='\n')
        (project/'ignored.js').write_text('invalid @@ ignored source')
        for directory in ['node_modules','.oyzu-build','dist','coverage']:
            (project/directory).mkdir()
            (project/directory/'invalid.json').write_text('invalid @@ private state')
        env = {**os.environ,'OYZU_NODE_QUALITY_HOME':str(ROOT/'tooling/images/node-quality')}

        def run(mode, success=True, extra=()):
            before = {p:p.read_bytes() for p in project.rglob('*') if p.is_file()}
            command = [str(args.cli.resolve()),'-C',str(project),'run',mode,'--',*extra] if args.cli else ['node',str(ROOT/'src/builders/node/runtime/quality.mjs'),f'biome-{mode}',*extra]
            result = subprocess.run(command,cwd=project,env=env,capture_output=True,text=True,encoding='utf-8',timeout=120)
            assert (result.returncode == 0) == success, (mode,result.stdout,result.stderr)
            if mode != 'format':
                assert before == {p:p.read_bytes() for p in project.rglob('*') if p.is_file()}, 'check mutated source'
            return result.stdout + result.stderr

        run('lint')
        run('format-check',False)
        assert 'do not accept extra arguments' in run('lint',False,extra=('biome-format',))
        run('format')
        run('format-check')
        assert source.read_text() == 'export const value = 42;\n'
        assert data.read_text() == '{"value":42}\n', 'implicit code formatting expanded to non-code metadata'
        assert (project/'ignored.js').read_text() == 'invalid @@ ignored source'
        source.write_text('debugger;\nexport const value = 42;\n',newline='\n')
        assert 'noDebugger' in run('lint',False)
        source.write_text('export const value = 42;\n',newline='\n')
        run('lint')
        # More than one command-line batch must preserve failure from an earlier batch.
        for index in range(120):
            (project/f'check-{index:03}-long-source-file-name.js').write_text('export const value = 42;\n',newline='\n')
        (project/'check-000-long-source-file-name.js').write_text('debugger;\n',newline='\n')
        assert 'noDebugger' in run('lint',False)
        package = json.loads(manifest.read_text())
        package['devDependencies'] = {'@biomejs/biome':'2.5.15'}
        manifest.write_text(json.dumps(package))
        assert 'Declared @biomejs/biome is not installed' in run('lint',False)
        # A native project dependency takes precedence even without usable defaults.
        (project/'check-000-long-source-file-name.js').write_text('export const value = 42;\n',newline='\n')
        subprocess.run(['node','-e',"require('node:fs').symlinkSync(process.argv[1],process.argv[2],process.platform==='win32'?'junction':'dir')",str(ROOT/'tooling/images/node-quality/node_modules/@biomejs'),str(project/'node_modules/@biomejs')],check=True)
        env['OYZU_NODE_QUALITY_HOME'] = str(project/'absent-defaults')
        run('lint')
    print('Native Biome passed: JSONC configuration, real lint/format failures, read-only checks, explicit code formatting, native ignores, private-state exclusions, argument guards, batching and installed tool precedence')


if __name__ == '__main__':
    main()
