"""Native host test bundles through the compiled CLI; no container engine."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    # Reuse schema/content checks without running captured-build scenarios.
    spec = importlib.util.spec_from_file_location('build_scenarios_runner', ROOT/'tooling/test-build-scenarios.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)

    with tempfile.TemporaryDirectory(prefix='oyzu direct test ') as directory:
        project = Path(directory)
        package = {'name':'native-evidence','version':'1.0.0','type':'module',
            'scripts':{'pretest':'node lifecycle.cjs pre','test':'node --test','posttest':'node lifecycle.cjs post'}}
        (project/'package.json').write_text(json.dumps(package))
        (project/'lifecycle.cjs').write_text("require('node:fs').appendFileSync('events.txt',process.argv[2]+'\\n');")
        (project/'src').mkdir()
        (project/'src/greeting.mjs').write_text("export function greeting() { return 'hello'; }\n")
        test = "import test from 'node:test'; import assert from 'node:assert/strict'; import {greeting} from './src/greeting.mjs'; test('greeting',()=>assert.equal(greeting(),'hello'));\n"
        (project/'greeting.test.mjs').write_text(test)
        # Ordinary native builds already own dist; direct tests preserve bytes.
        (project/'dist/assets').mkdir(parents=True)
        (project/'dist/assets/app.js').write_text('native application output')

        def invoke(*argv, success=True):
            result = subprocess.run([str(cli),'-C',str(project),'--json',*argv],capture_output=True,text=True,timeout=90)
            assert (result.returncode==0)==success, (argv,result.stdout,result.stderr)
            return json.loads(result.stdout) if result.stdout.strip() else None

        outcomes = invoke('run','test')
        assert len(outcomes)==1 and outcomes[0]['status']=='succeeded'
        manifest = harness.validate(project/'dist')
        assert manifest['status']=='succeeded' and not manifest['artifacts']
        assert (project/'dist/assets/app.js').read_text()=='native application output'
        assert any(e['path']=='assets/app.js' for e in manifest['extensions']['oyzu.dev/host-outputs'])
        assert manifest['extensions']['oyzu.dev/invocation']['execution']=='host'
        assert [a['id'] for a in manifest['actions']]==['project:test']
        assert (project/'events.txt').read_text().splitlines()==['pre','post']
        reports = {r['kind']:r for r in manifest['reports']}
        assert reports['test']['summary']['total']==1
        assert reports['coverage']['summary']['covered']>0
        coverage = (project/'dist'/reports['coverage']['path']).read_text()
        assert 'greeting.mjs' in coverage and 'greeting.test.mjs' not in coverage
        plan = json.loads((project/'dist/plan.json').read_text())
        assert all(a['network']=='host' and not a['cacheable'] for a in plan['actions'])
        assert not plan['tools']
        invoke('inspect','dist')
        (project/'dist/assets/app.js').write_text('tampered')
        invoke('inspect','dist',success=False)
        (project/'dist/assets/app.js').write_text('native application output')
        original_manifest = (project/'dist/manifest.json').read_bytes()
        forged = json.loads(original_manifest)
        forged['extensions']['oyzu.dev/invocation']['execution']='isolated'
        (project/'dist/manifest.json').write_text(json.dumps(forged))
        invoke('inspect','dist',success=False)
        (project/'dist/manifest.json').write_bytes(original_manifest)
        # Fresh output roots: a failed test retains current evidence, not the
        # earlier passing bundle. Native npm posttest remains success-only.
        (project/'greeting.test.mjs').write_text(test.replace("greeting(),'hello'", "greeting(),'wrong'"))
        invoke('run','test',success=False)
        failed = harness.validate(project/'dist')
        assert failed['status']=='failed' and not failed['artifacts']
        assert next(r for r in failed['reports'] if r['kind']=='test')['summary']['failed']==1
        assert len(list((project/'.oyzu/history').iterdir()))==2
        assert (project/'dist/assets/app.js').read_text()=='native application output'
        assert (project/'events.txt').read_text().splitlines()==['pre','post','pre']
        invoke('inspect','dist')
        (project/'greeting.test.mjs').write_text(test)
        # Hooks see the assigned paths, and collection waits for the post-hook.
        post = "const fs=require('node:fs'); if(!fs.existsSync(process.env.OYZU_TEST_REPORT))process.exit(3); fs.appendFileSync('events.txt','oyzu-post\\n');"
        (project/'oyzu.toml').write_text('[tasks."project:post_test"]\nargv='+json.dumps(['node','-e',post])+'\n')
        invoke('run','test')
        hooked = harness.validate(project/'dist')
        assert [a['id'] for a in hooked['actions']]==['project:test','project:post_test']
        assert (project/'events.txt').read_text().splitlines()[-1]=='oyzu-post'
        # TOML shell shorthand recognizes only exact native commands on each OS.
        (project/'oyzu.toml').write_text('[tasks."project:test"]\nrun="npm run test"\n')
        invoke('run','test')
        assert harness.validate(project/'dist')['status']=='succeeded'
        # Custom bodies retain required reports. Exit zero alone is insufficient.
        (project/'oyzu.toml').write_text('[tasks."project:test"]\nargv=["node","-e","process.exit(0)"]\n')
        invoke('run','test',success=False)
        missing = harness.validate(project/'dist')
        assert missing['status']=='failed' and len(missing['reports'])==2
        assert all(r['status']=='invalid' for r in missing['reports'])
        # Default native runner needs no package script or Oyzu configuration.
        (project/'oyzu.toml').unlink()
        package.pop('scripts')
        (project/'package.json').write_text(json.dumps(package))
        invoke('run','test','--','--test-name-pattern=greeting')
        implicit = harness.validate(project/'dist')
        assert implicit['status']=='succeeded'
        assert next(r for r in implicit['reports'] if r['kind']=='test')['summary']['total']==1
        # Successful native tests cannot mask a failing hook. A failed pre-hook
        # blocks the test; a failed post-hook still retains current reports.
        for hook in ['pre_test','post_test']:
            (project/'oyzu.toml').write_text(f'[tasks."project:{hook}"]\nargv=["node","-e","process.exit(7)"]\n')
            invoke('run','test',success=False)
            hooked = harness.validate(project/'dist')
            assert hooked['status']=='failed'
            if hook=='pre_test':
                assert hooked['actions'][-1]['status']=='blocked' and not hooked['reports']
            else:
                assert hooked['actions'][-1]['status']=='failed'
                assert next(r for r in hooked['reports'] if r['kind']=='test')['summary']['total']==1
        # Exact overrides can redirect reports into new nested directories.
        (project/'oyzu.toml').write_text('[tasks."project:test"]\nargv=["node","--test"]\nreports=[{kind="test",format="junit",path="custom/nested/junit.xml"}]\n')
        invoke('run','test')
        redirected = harness.validate(project/'dist')
        assert redirected['status']=='succeeded' and (project/'custom/nested/junit.xml').is_file()
        # Malformed evidence produced by a post-hook must be retained and fail.
        corrupt = "require('node:fs').writeFileSync(process.env.OYZU_TEST_REPORT,'<malformed>')"
        (project/'oyzu.toml').write_text('[tasks."project:post_test"]\nargv='+json.dumps(['node','-e',corrupt])+'\n')
        invoke('run','test',success=False)
        malformed = harness.validate(project/'dist')
        invalid = next(r for r in malformed['reports'] if r['kind']=='test')
        assert invalid['status']=='invalid'
        assert (project/'dist'/invalid['path']).read_text()=='<malformed>'
        # Launch failures still finalize a failed test-only manifest.
        (project/'oyzu.toml').write_text('[tasks."project:test"]\nargv=["oyzu-definitely-missing-executable"]\n')
        invoke('run','test',success=False)
        unavailable = harness.validate(project/'dist')
        assert unavailable['status']=='failed' and unavailable['actions'][0]['exitCode']!=0
        # Configured coverage gates apply even when all native tests pass.
        (project/'src/greeting.mjs').write_text("export function greeting(who='hello') {\n if (who === 'other') {\n  return 'untested';\n }\n return who;\n}\n")
        (project/'oyzu.toml').write_text('[checks]\ncoverageMinimum=100\n[env]\nPROBE_PRIVATE="do-not-export-this-value"\n')
        assert invoke('run','test',success=False)
        gated = harness.validate(project/'dist')
        assert gated['runId']!=unavailable['runId']
        assert gated['status']=='failed'
        assert next(r for r in gated['reports'] if r['kind']=='test')['summary']['failed']==0
        assert 'do-not-export-this-value' not in (project/'dist/plan.json').read_text()
        plan = json.loads((project/'dist/plan.json').read_text())
        assert plan['actions'][0]['extensions']['oyzu.dev/coverage-minimum']==100
        assert plan['actions'][0]['extensions']['oyzu.dev/configuration-digest']
        (project/'src/greeting.mjs').write_text("export function greeting() { return 'hello'; }\n")
        # Preserve the existing bundle when explicit declared outputs are stale.
        (project/'old.xml').write_text('<testsuite><testcase/></testsuite>')
        (project/'oyzu.toml').write_text('[tasks."project:test"]\nargv=["node","--test"]\nreports=[{kind="test",format="junit",path="old.xml"}]\n')
        previous = (project/'dist/manifest.json').read_bytes()
        invoke('run','test',success=False)
        assert (project/'dist/manifest.json').read_bytes()==previous
        assert (project/'old.xml').read_text()=='<testsuite><testcase/></testsuite>'
    print('Direct Node tests: native scripts/lifecycle, implicit runner, JUnit/application coverage, host provenance, native dist preservation/tampering, history, failed/missing/malformed evidence, hook failures, redirected reports, launch failures, arguments and stale-output rejection passed.')


if __name__=='__main__':
    main()
