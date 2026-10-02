"""No-change affected build through the real CLI, without native build tooling."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
from build_scenarios.affected import initialize

parser=argparse.ArgumentParser()
parser.add_argument('--cli',type=Path,required=True)
cli=parser.parse_args().cli.resolve()
root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='oyzu affected ') as directory:
    project=Path(directory)/'project'
    shutil.copytree(root/'examples/builds/affected-graph/variants/targets',project)
    baseline=initialize(Path(directory))
    def invoke(*args,success=True):
        result=subprocess.run([str(cli),'--root',str(project),*args],capture_output=True,text=True,timeout=120)
        assert (result.returncode==0)==success,result.stdout+result.stderr
        return json.loads(result.stdout) if success else result.stderr
    plan=invoke('build','--affected',baseline,'--plan')
    assert not plan['targets'] and not plan['actions'] and not (project/'dist').exists()
    selection=plan['extensions']['oyzu.dev/selection']
    assert selection['mode']=='affected' and selection['requested']==selection['selected']==[]
    assert selection['affected']['baselineCommit']==baseline
    assert selection['affected']['fallback'] is None and len(selection['excluded'])==4
    manifest=invoke('build','--affected',baseline)
    assert manifest['status']=='succeeded' and not manifest['targets'] and not manifest['artifacts']
    invoke('inspect','dist')
    assert 'cannot be used with' in invoke('build','api','--affected',baseline,success=False)
print('Real CLI affected no-change plan/build/inspection and conflicting selection passed')
