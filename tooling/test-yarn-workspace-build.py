"""Native Yarn capture -> offline workspace build -> snapshot packages/reports.

This checks the native adapter. Compiled-CLI isolated acceptance runs in CI.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]


def main():
    native = ROOT/'tooling/images/node-yarn/node_modules/yarn/bin/yarn.js'
    with tempfile.TemporaryDirectory(prefix='oyzu Yarn workspace ') as directory:
        base = Path(directory)
        runtime = base/'runtime'
        shutil.copytree(ROOT/'src/builders/node/runtime', runtime)
        shutil.copyfile(runtime/'archive.mjs', runtime/'node-archive.mjs')
        shutil.copyfile(runtime/'quality.mjs', runtime/'node-quality.mjs')
        shutil.copyfile(ROOT/'src/broker/runtime/transport.mjs', runtime/'broker_transport.mjs')
        wrapper = runtime/'capture.mjs'
        wrapper.write_text(f"import {{profile}} from './yarn.mjs'; import {{run}} from './manager-runtime.mjs'; profile.command=[process.execPath,{json.dumps(str(native))}]; await run(profile);\n",encoding='utf-8')
        env = dict(os.environ, OYZU_YARN_LOCKFILE=str(native.parents[2]/'@yarnpkg/lockfile'),
                   OYZU_NODE_QUALITY_HOME=str(ROOT/'tooling/images/node-quality'),
                   PATH=str(native.parents[2]/'.bin')+os.pathsep+os.environ['PATH'], OYZU_TARGET='project')
        source = base/'source'
        shutil.copytree(ROOT/'examples/builds/node-workspace/variants/yarn-classic',source)
        subprocess.run(['node',str(native),'install','--offline','--non-interactive','--ignore-scripts'],cwd=source,env=env,check=True,capture_output=True)
        originals={p.relative_to(source):p.read_bytes() for p in source.rglob('*') if p.is_file() and 'node_modules' not in p.parts}
        acquired=base/'acquired'
        shutil.copytree(source,acquired,ignore=shutil.ignore_patterns('node_modules'))
        captured=base/'dependencies'
        captured.mkdir()

        def invoke(script,args,project,success=True):
            result=subprocess.run(['node',str(runtime/script),*args],cwd=project,env=env,capture_output=True,text=True,encoding='utf-8',timeout=180)
            assert (result.returncode==0)==success,(args,result.stdout,result.stderr)
            return result

        invoke('capture.mjs',['acquire',str(captured),str(acquired)],acquired)
        inventory=json.loads((captured/'inventory.json').read_text())
        metadata=inventory['workspaces']
        assert len(metadata['members'])==2 and not inventory['packages']
        members=sorted(metadata['members'],key=lambda m:len(m['dependencies']))
        specification={'rootVersion':'0.1.0-dev.g0123456789ab','rootArtifact':None,'rootScripts':{},
                       'rootDependencies':metadata['rootDependencies'],'rootFramework':'node-test',
                       'rootQuality':{'linter':'eslint','formatter':'prettier','excludes':[m['path'] for m in members]},
                       'nodeTestArguments':['--experimental-test-coverage','--test-coverage-exclude=**/*.test.*','--test-reporter=junit','--test-reporter-destination=__OYZU_TEST_REPORT__','--test-reporter=lcov','--test-reporter-destination=__OYZU_COVERAGE_REPORT__'],
                       'modules':[{**m,'id':m['name'].split('/')[-1],'version':'0.1.0-dev.g0123456789ab',
                                   'filename':m['name'].removeprefix('@').replace('/','-')+'-0.1.0-dev.g0123456789ab.tgz',
                                   'framework':'node-test','testExcludes':[],
                                   'quality':{'linter':'eslint','formatter':'prettier','excludes':[]}} for m in members]}
        encoded=json.dumps(specification,separators=(',',':'))
        env['OYZU_NODE_WORKSPACE_PLAN']=hashlib.sha256(encoded.encode()).hexdigest()
        project=base/'execution'
        shutil.copytree(source,project,ignore=shutil.ignore_patterns('node_modules'))
        invoke('yarn-workspace-build.mjs',['project',encoded],project)
        invoke('capture.mjs',['install',str(captured),str(project)],project)
        for stage in ['build','test','lint','format-check']:
            invoke('yarn-workspace-build.mjs',[stage],project)
        output=base/'out'
        invoke('yarn-workspace-build.mjs',['package','unused',str(output)],project)
        artifacts=list((output/'project/artifacts').glob('*.tgz'))
        assert len(artifacts)==2
        for artifact in artifacts:
            with tarfile.open(artifact) as archive:
                package=json.load(archive.extractfile('package/package.json'))
                assert package['version']=='0.1.0-dev.g0123456789ab'
                if package['name'].endswith('/app'):
                    assert package['dependencies']['@oyzu-example/shared']==package['version']
        for m in specification['modules']:
            reports=project/'.oyzu-build/reports'/m['id']
            assert ET.parse(reports/'junit.xml').findall('.//testcase')
            assert 'DA:' in (reports/'coverage.lcov').read_text()
        (project/'packages/shared/failure.test.mjs').write_text("import test from 'node:test'; test('failure',()=>{throw Error('expected')});\n")
        invoke('yarn-workspace-build.mjs',['test'],project,False)
        assert ET.parse(project/'.oyzu-build/reports/shared/junit.xml').findall('.//failure')
        assert originals=={p:(source/p).read_bytes() for p in originals}
        print('Yarn workspace: native capture, fresh offline links, snapshot packages/local references, real tests/coverage, quality and failure evidence passed')


if __name__=='__main__':
    main()
