"""Exercise compiled-CLI workspace tests with real Node, Jest and Vitest runners."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--jest-modules', type=Path, default=ROOT/'tooling/fixtures/jest/node_modules')
    parser.add_argument('--vitest-modules', type=Path, default=ROOT/'tooling/fixtures/vitest/node_modules')
    args = parser.parse_args()
    cli = args.cli.resolve()
    with tempfile.TemporaryDirectory(prefix='oyzu workspace tests ') as temporary:
        base = Path(temporary)
        project = base/'project'
        project.mkdir()
        log = base/'executions.jsonl'
        env = {**os.environ, 'OYZU_WORKSPACE_TEST_LOG':str(log)}
        manifest = project/'package.json'
        package = {'name':'workspace-tests','version':'1.0.0','workspaces':['packages/*','packages/node/nested']}
        manifest.write_text(json.dumps(package))
        marker = "require('node:fs').appendFileSync(process.env.OYZU_WORKSPACE_TEST_LOG,JSON.stringify(process.argv.slice(2))+'\\n');\n"
        (project/'marker.cjs').write_text(marker)
        for name in ['node','jest','vitest','node/nested']:
            member = project/'packages'/name
            member.mkdir(parents=True, exist_ok=True)
            pkg = {'name':name.replace('/','-'),'version':'1.0.0','private':True}
            if name == 'node/nested':
                pkg['scripts'] = {stage:f'node ../../../marker.cjs {stage}' for stage in ['pretest','test','posttest']}
            (member/'package.json').write_text(json.dumps(pkg))
        native = (ROOT/'src/builders/node/runtime/npm-native.mjs').as_uri()
        subprocess.run(['node','--input-type=module','-e',f"import {{npm}} from {json.dumps(native)}; npm(['install','--ignore-scripts'],process.cwd(),process.argv[1]);",str(base/'cache')],cwd=project,check=True,capture_output=True,text=True)

        def write_test(path, name, framework):
            imports = {'node':"const {test}=require('node:test');",'jest':'','vitest':"import {test} from 'vitest'; import {createRequire} from 'node:module'; const require=createRequire(import.meta.url);"}[framework]
            path.write_text(imports + f"test('{name}',()=>{{require('node:fs').appendFileSync(process.env.OYZU_WORKSPACE_TEST_LOG,JSON.stringify([{json.dumps(name)}])+'\\n')}});\n")

        write_test(project/'root.test.cjs','root','node')
        write_test(project/'packages/node/member.test.cjs','node','node')
        write_test(project/'packages/jest/member.test.js','jest','jest')
        write_test(project/'packages/vitest/member.test.mjs','vitest','vitest')
        (project/'packages/jest/jest.config.cjs').write_text("module.exports={maxWorkers:1};\n")
        (project/'packages/vitest/vitest.config.mjs').write_text("export default {test:{maxWorkers:1}};\n")
        for name, modules in [('jest',args.jest_modules),('vitest',args.vitest_modules)]:
            assert modules.is_dir(), f'Provision native {name} dependencies first: {modules}'
            subprocess.run(['node','-e',"require('node:fs').symlinkSync(process.argv[1],process.argv[2],process.platform==='win32'?'junction':'dir')",str(modules.resolve()),str(project/f'packages/{name}/node_modules')],check=True)
        for directory in ['.oyzu-build','packages/node/nested']:
            (project/directory).mkdir(exist_ok=True)
            (project/directory/'excluded.test.cjs').write_text("throw Error('A parent executed a test owned by a child');\n")

        def run(*argv, success=True, extra_env=None):
            result = subprocess.run([str(cli),'-C',str(project),*argv],env={**env,**(extra_env or {})},capture_output=True,text=True,encoding='utf-8',timeout=120)
            assert (result.returncode == 0) == success, result.stdout + result.stderr
            return result

        def entries():
            result = [json.loads(line) for line in log.read_text(encoding='utf-8').splitlines()]
            log.unlink()
            return result

        listed = json.loads(run('run','list','--json',extra_env={'PATH':''}).stdout)
        assert listed['project:test']['availability'] is None and not log.exists()
        run('run','test')
        result = entries()
        assert sorted(x[0] for x in result) == sorted(['root','node','jest','vitest','pretest','test','posttest']), result
        assert result.index(['pretest']) < result.index(['test']) < result.index(['posttest'])
        # A real member assertion failure must fail the aggregate, but later suites still run.
        failed = project/'packages/node/failure.test.cjs'
        failed.write_text("require('node:test').test('fails',()=>{throw Error('expected member failure')});\n")
        run('run','test',success=False)
        assert sorted(x[0] for x in entries()) == sorted(x[0] for x in result)
        failed.unlink()
        # Explicit root scripts own the aggregate and preserve native arguments/lifecycle.
        package['scripts'] = {stage:f'node marker.cjs root-{stage}' for stage in ['pretest','test','posttest']}
        manifest.write_text(json.dumps(package))
        forwarded = ['with spaces','','semi;colon','snowman-\u2603']
        run('run','test','--',*forwarded)
        assert entries() == [['root-pretest'],['root-test',*forwarded],['root-posttest']]
        (project/'oyzu.toml').write_text('[tasks."project:test"]\nargv=["node","marker.cjs","override"]\n')
        run('run','test')
        assert entries() == [['override']]
        (project/'oyzu.toml').unlink()
        package.pop('scripts')
        manifest.write_text(json.dumps(package))
        (project/'packages/node/member.test.cjs').unlink()
        assert 'No package Node tests found' in run('run','test',success=False).stderr
    print('Workspace test composition passed: native Node/Jest/Vitest, nested ownership, lifecycle, root precedence, overrides, exact arguments, real failures and missing tests')


if __name__ == '__main__':
    main()
