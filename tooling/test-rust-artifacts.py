"""Real Cargo artifact messages, feature gates and native executable staging."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ADAPTER = ROOT/'src/builders/rust/runtime/build.py'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cargo', default='cargo')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='oyzu cargo outputs ') as temporary:
        base = Path(temporary).resolve()
        project = base/'project'
        shutil.copytree(ROOT/'examples/builds/rust-app/project',project)
        shutil.copyfile(ROOT/'examples/builds/rust-app/variants/features.Cargo.toml',project/'Cargo.toml')
        shutil.copyfile(ROOT/'examples/builds/rust-app/variants/extra.rs',project/'src/extra.rs')
        env = {**os.environ,'CARGO_TARGET_DIR':str(project/'.oyzu-build/target'), 'CARGO_NET_OFFLINE':'true'}
        bindings = base/'binaries.json'

        def native(*argv):
            result = subprocess.run([args.cargo,*argv],cwd=project,env=env,capture_output=True,text=True,encoding='utf-8',timeout=180)
            assert result.returncode==0, (result.stdout,result.stderr)
            return result.stdout

        def select(names):
            metadata = json.loads(native('metadata','--format-version','1','--locked','--offline'))
            package, = metadata['packages']
            bindings.write_text(json.dumps({'schemaVersion':1,'binaries':[{'packageId':package['id'],'name':name} for name in names]}))

        staging = project/'.oyzu-build/target/oyzu-binaries'

        def build(success=True, extra=()):
            result = subprocess.run([sys.executable,str(ADAPTER),str(bindings),args.cargo,'build',
                                     '--workspace','--release','--locked','--offline','--message-format=json-render-diagnostics',*extra],
                                    cwd=project,env=env,capture_output=True,text=True,encoding='utf-8',timeout=180)
            assert (result.returncode==0)==success, (result.stdout,result.stderr)
            if not success:
                assert not list(staging.iterdir()), 'failed build reused staged executables'
            return result

        select(['oyzu-greeting-example'])
        first = build()
        events = [json.loads(line) for line in first.stdout.splitlines() if line.startswith('{')]
        artifacts = [e for e in events if e.get('reason')=='compiler-artifact' and e.get('executable')]
        assert len(artifacts)==1 and (staging/'0').read_bytes()==Path(artifacts[0]['executable']).read_bytes()
        executable = base/('published.exe' if os.name=='nt' else 'published')
        shutil.copy2(staging/'0',executable)
        assert subprocess.check_output([str(executable)],text=True).strip()=='Hello, Oyzu!'
        second = build()
        assert any(json.loads(line).get('fresh') is True for line in second.stdout.splitlines() if line.startswith('{'))
        listed = native('package','--locked','--offline','--allow-dirty','--list')
        assert 'oyzu-binaries' not in listed and '.oyzu-build' not in listed, listed
        # A required feature is enabled by native Cargo defaults, without Oyzu configuration.
        manifest = project/'Cargo.toml'
        disabled = manifest.read_text()
        manifest.write_text(disabled.replace('extra = []','extra = []\ndefault = ["extra"]'))
        select(['oyzu-greeting-example','extra'])
        build()
        shutil.copy2(staging/'1',executable)
        assert subprocess.check_output([str(executable)],text=True).strip()=='Optional command'
        manifest.write_text(disabled)
        build(success=False)  # stale gated output cannot satisfy a new invocation
        select(['oyzu-greeting-example'])
        build(success=False,extra=('--target-dir',str(base/'unexpected-target')))
        (project/'src/main.rs').write_text('compile_error!("native compile failure");\n')
        build(success=False)
        print('Native Cargo artifact capture passed: feature-disabled/enabled outputs, fresh cache, executable identity, archive exclusion, unexpected paths, missing outputs and compiler failures')


if __name__ == '__main__':
    main()
