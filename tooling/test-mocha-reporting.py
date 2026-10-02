"""Exercise native Mocha/c8 reporting without claiming isolated execution."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


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
    print('Native Mocha discovery, configuration, reporter composition, lifecycle, JUnit/LCOV, failed assertions, coverage thresholds and no-script task passed.')


if __name__=='__main__':
    main()
