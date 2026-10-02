"""Exercise native Mocha/c8 reporting without claiming isolated execution."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


def workspace(cli, base, npm):
    project = base/'workspace'
    shutil.copytree(ROOT/'tooling/fixtures/mocha-workspace', project)
    subprocess.run([npm,'ci','--ignore-scripts','--no-audit','--no-fund'],cwd=project,check=True)
    runtime = ROOT/'src/builders/node/runtime'
    env = {**os.environ, 'OYZU_NODE_REPORTING_HOME':str(ROOT/'tooling/images/node-quality'),
           'OYZU_NODE_QUALITY_HOME':str(ROOT/'tooling/images/node-quality')}
    for stage in ['lint','format-check']:
        result = subprocess.run([cli,'run',stage],cwd=project,env=env,capture_output=True,text=True)
        assert result.returncode==0, result.stdout+result.stderr
    result = subprocess.run([cli,'run','test'],cwd=project,env=env,capture_output=True,text=True)
    assert result.returncode==0, result.stdout+result.stderr
    assert result.stdout.count('implicit member only')==1
    assert result.stdout.count('root only')==0, 'root native dot reporter should be retained'
    # Runtime contract probe. The captured CLI acceptance suite separately
    # verifies that the Rust planner supplies this per-package specification.
    modules = []
    for name in ['implicit','scripted']:
        path = f'packages/{name}'
        package = json.loads((project/path/'package.json').read_text())
        modules.append({'id':name,'path':path,'name':package['name'],
                        'scripts':package.get('scripts',{}),'framework':'mocha','testExcludes':[]})
    spec = {'rootScripts':{},'rootArtifact':{'path':'.'},'rootFramework':'mocha','modules':modules}
    state = project/'.oyzu-build'
    state.mkdir()
    encoded = json.dumps(spec)
    (state/'plan.json').write_text(encoded)
    env['OYZU_NPM_WORKSPACE_PLAN'] = hashlib.sha256(encoded.encode()).hexdigest()
    def run(success=True):
        shutil.rmtree(state/'reports', ignore_errors=True)
        result = subprocess.run(['node',str(runtime/'npm-workspace-build.mjs'),'test'],cwd=project,env=env,
                                capture_output=True,text=True,timeout=120)
        assert (result.returncode==0)==success, result.stdout+result.stderr
        reports = {}
        for name in ['root','implicit','scripted']:
            report = state/'reports'/name
            junit = ET.parse(report/'junit.xml')
            assert len(junit.findall('.//testcase'))==1, name
            assert 'index.js' in (report/'coverage.lcov').read_text()
            reports[name] = junit
        assert '"stats"' in result.stdout, 'member JSON reporter lost to root configuration'
        return reports
    run()
    member = project/'packages/implicit/test/member.js'
    original = member.read_text()
    member.write_text(original.replace('value, 11','value, 99'))
    reports = run(False)
    assert len(reports['implicit'].findall('.//failure'))==1
    assert not reports['scripted'].findall('.//failure') and not reports['root'].findall('.//failure')
    member.write_text(original)
    # A declared root script owns the aggregate and can intentionally include
    # member suites; it must not also trigger individual member invocations.
    package_path = project/'package.json'
    package = json.loads(package_path.read_text())
    package['scripts']={'test':'mocha'}
    package_path.write_text(json.dumps(package))
    spec['rootScripts']=package['scripts']
    encoded = json.dumps(spec)
    (state/'plan.json').write_text(encoded)
    env['OYZU_NPM_WORKSPACE_PLAN']=hashlib.sha256(encoded.encode()).hexdigest()
    shutil.rmtree(state/'reports')
    result = subprocess.run(['node',str(runtime/'npm-workspace-build.mjs'),'test'],cwd=project,env=env,
                            capture_output=True,text=True,timeout=120)
    assert result.returncode==0, result.stdout+result.stderr
    assert [p.name for p in (state/'reports').iterdir()]==['root']
    assert len(ET.parse(state/'reports/root/junit.xml').findall('.//testcase'))==3


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cli', type=Path, required=True)
    args = parser.parse_args()
    cli = str(args.cli.resolve())
    with tempfile.TemporaryDirectory(prefix='oyzu mocha ') as temporary:
        project = Path(temporary)/'project'
        shutil.copytree(ROOT/'tooling/fixtures/mocha', project, ignore=shutil.ignore_patterns('node_modules'))
        npm = shutil.which('npm.cmd' if os.name=='nt' else 'npm')
        subprocess.run([npm, 'ci', '--ignore-scripts', '--no-audit', '--no-fund'], cwd=project, check=True)
        out = Path(temporary)/'reports'
        env = {**os.environ, 'OYZU_TEST_REPORT':str(out/'junit.xml'), 'OYZU_COVERAGE_REPORT':str(out/'coverage.lcov'),
               'OYZU_NODE_REPORTING_HOME':str(ROOT/'tooling/images/node-quality'),
               'OYZU_NODE_QUALITY_HOME':str(ROOT/'tooling/images/node-quality')}
        for operation in ['lint', 'format-check']:
            checked = subprocess.run([cli,'run',operation],cwd=project,env=env,capture_output=True,text=True)
            assert checked.returncode==0, checked.stdout+'\n'+checked.stderr
        adapter = ROOT/'src/builders/node/runtime/mocha.mjs'
        config = project/'.mocharc.cjs'
        config.write_text("const fs=require('node:fs');fs.appendFileSync('config-loads.txt','once\\n');module.exports={reporter:'json'};\n")
        package_file = project/'package.json'
        package = json.loads(package_file.read_text())
        for hook in ['pretest', 'posttest']:
            package['scripts'][hook]=f'''node -e "require('node:fs').appendFileSync('{hook}.txt','once\\n')"'''
        package_file.write_text(json.dumps(package))
        tasks = json.loads(subprocess.check_output([cli, 'run', 'list', '--json'], cwd=project, text=True))
        assert tasks['project:test']['argv']==['npm','run','test']
        assert not (project/'config-loads.txt').exists(), 'discovery evaluated native config'

        def run(success=True):
            if out.exists(): shutil.rmtree(out)
            result = subprocess.run(['node',str(adapter),'npm','run','test','--'], cwd=project, env=env,
                                    capture_output=True, text=True, timeout=120)
            assert (result.returncode==0)==success, result.stdout+'\n'+result.stderr
            return result

        result = run()
        assert '"pending"' in result.stdout, 'native JSON reporter was not retained'
        report = ET.parse(out/'junit.xml')
        assert len(report.findall('.//testcase'))==2 and len(report.findall('.//skipped'))==1
        assert 'greets <Oyzu> & friends' in [t.attrib['name'] for t in report.findall('.//testcase')]
        assert 'src/greeting.js' in (out/'coverage.lcov').read_text().replace('\\','/')
        assert (project/'config-loads.txt').read_text()=='once\n', repr((project/'config-loads.txt').read_text()) + '\n' + result.stdout + '\n' + result.stderr
        for hook in ['pretest','posttest']:
            assert (project/f'{hook}.txt').read_text()=='once\n'
        test = project/'test/greeting.test.js'
        original = test.read_text()
        test.write_text(original.replace('"Hello, Oyzu!"','"wrong"'))
        run(False)
        assert len(ET.parse(out/'junit.xml').findall('.//failure'))==1
        assert (out/'coverage.lcov').is_file()
        test.write_text(original)
        (project/'.c8rc.json').write_text(json.dumps({'check-coverage':True,'branches':100}))
        run(False)
        assert not ET.parse(out/'junit.xml').findall('.//failure')
        assert (out/'coverage.lcov').is_file()
        (project/'.c8rc.json').unlink()
        config.write_text("throw Error('native-config-failure');\n")
        result = run(False)
        assert 'native-config-failure' in result.stderr and not (out/'junit.xml').exists()
        package['scripts'].pop('test')
        package_file.write_text(json.dumps(package))
        config.write_text("module.exports={reporter:'dot'};\n")
        tasks = json.loads(subprocess.check_output([cli,'run','list','--json'],cwd=project,text=True))
        assert tasks['project:test']['argv']==['node','node_modules/mocha/bin/mocha.js']
        subprocess.run([cli,'run','test'],cwd=project,check=True,capture_output=True)
        workspace(cli, Path(temporary), npm)
    print('Native Mocha discovery, configuration, reporter composition, lifecycle, JUnit/LCOV, failures, thresholds, implicit/scripted workspace suites, package scope and root ownership passed.')


if __name__=='__main__':
    main()
