"""Git-selected native target builds; action-cache reuse is separate."""
import shutil
import subprocess


def initialize(project):
    def git(*args):
        return subprocess.run(['git','-C',str(project),'-c','core.autocrlf=false','-c','commit.gpgsign=false',
            '-c','user.name=Oyzu Fixture','-c','user.email=fixture@example.invalid',*args],
            check=True,capture_output=True,text=True).stdout.strip()
    git('init','--template=','-b','main')
    git('add','.')
    git('commit','-m','baseline')
    return git('rev-parse','HEAD')


def verify(root, base, invoke, validate, source_files, verified):
    project=base/'affected-targets'
    shutil.copytree(root/'examples/builds/affected-graph/variants/targets',project)
    baseline=initialize(project)
    invoke(project,'build','--affected',baseline)
    manifest=validate(project/'dist')
    assert manifest['status']=='succeeded' and not manifest['actions'] and not manifest['artifacts']
    assert manifest['extensions']['oyzu.dev/selection']['selected']==[]
    invoke(project,'inspect','dist')
    source=project/'api/src/greeting.mjs'
    original=source.read_bytes()
    source.write_bytes(original+b'\n// API input changed.\n')
    before=source_files(project)
    invoke(project,'build','--affected',baseline)
    manifest=validate(project/'dist')
    assert manifest['status']=='succeeded' and source_files(project)==before
    selection=manifest['extensions']['oyzu.dev/selection']
    assert selection['mode']=='affected' and selection['requested']==['api']
    assert selection['selected']==['api','shared']
    assert len(manifest['artifacts'])==2 and all('-dev.g' in a['version'] for a in manifest['artifacts'])
    assert {r['target'] for r in manifest['reports'] if r['kind']=='test'}=={'api','shared'}
    for owner in ['api','shared']:
        for stage in ['build','test','lint','format-check','package']:
            assert next(a for a in manifest['actions'] if a['id']==f'{owner}:{stage}')['status']=='succeeded'
    invoke(project,'inspect','dist')
    source.write_bytes(original)
    shared=project/'shared/src/greeting.mjs'
    shared.write_bytes(shared.read_bytes()+b'\n// Shared input changed.\n')
    invoke(project,'build','--affected',baseline)
    manifest=validate(project/'dist')
    assert manifest['status']=='succeeded'
    assert manifest['extensions']['oyzu.dev/selection']['selected']==['api','shared','web']
    assert len(manifest['artifacts'])==3
    invoke(project,'inspect','dist')
    verified.append('Affected targets: local Git baseline, no-change bundle, dependency closure, consumers, real snapshot artifacts/required stages and inspected selection evidence')
