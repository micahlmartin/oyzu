"""Native Cargo mixed workspace/registry packaging; no internet or cached inputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import runpy
import subprocess
import sys
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ADAPTER = ROOT/'src/builders/rust/runtime/package.py'


def files(root):
    return {p.relative_to(root):p.read_bytes() for p in root.rglob('*')
            if p.is_file() and '.oyzu-build' not in p.relative_to(root).parts}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--cargo', default='cargo')
    args = parser.parse_args()
    cargo = os.path.abspath(shutil.which(args.cargo) or args.cargo)
    with tempfile.TemporaryDirectory(prefix='oyzu mixed Cargo packages ') as temporary:
        base = Path(temporary)
        home = base/'cargo-home'
        home.mkdir()
        config = home/'config.toml'
        config.write_text('# fresh native Cargo home\n')
        env = {**os.environ, 'CARGO_HOME':str(home), 'CARGO_NET_OFFLINE':'true'}
        fixture = base/'fixture'
        (fixture/'src').mkdir(parents=True)
        (fixture/'Cargo.toml').write_text('[package]\nname="fixture-dep"\nversion="1.0.0"\nedition="2021"\n')
        (fixture/'src/lib.rs').write_text('pub fn answer() -> u32 { 42 }\n')

        def native(project, *argv):
            result = subprocess.run([cargo,*argv],cwd=project,env=env,
                                    capture_output=True,text=True,timeout=180)
            assert result.returncode == 0, (result.stdout,result.stderr)

        # Cargo itself creates the input archive and index; no private cache format.
        env['CARGO_TARGET_DIR'] = str(fixture/'.oyzu-build/target')
        native(fixture,'generate-lockfile','--offline')
        native(fixture,'package','--locked','--offline','--allow-dirty','--registry','crates-io')
        registry = base/'captured-registry'
        shutil.copytree(fixture/'.oyzu-build/target/package/tmp-registry',registry)
        config.write_text('[source.crates-io]\nreplace-with="captured"\n'
                          f'[source.captured]\nlocal-registry={json.dumps(registry.as_posix())}\n')
        project = base/'project'
        shutil.copytree(ROOT/'examples/builds/rust-workspace/project',project)
        version = '0.1.0-dev.g123456789abc'
        for manifest in project.glob('*/Cargo.toml'):
            text = manifest.read_text().replace('version = "0.1.0"',f'version = "{version}"')
            for name in ['core','macros']:
                text = text.replace(f'path = "../{name}"',f'path = "../{name}", version = "={version}"')
            if manifest.parent.name == 'core':
                text += '\n[dependencies]\nfixture-dep="=1.0.0"\n'
            manifest.write_text(text)
        source = project/'core/src/lib.rs'
        source.write_text(source.read_text()+'\n#[test]\nfn external_dependency() { assert_eq!(fixture_dep::answer(),42); }\n')
        env['CARGO_TARGET_DIR'] = str(project/'.oyzu-build/target')
        native(project,'generate-lockfile','--offline')
        native(project,'test','--workspace','--locked','--offline')
        before, captured, configuration = files(project), files(registry), config.read_bytes()
        inventory = [{'name':f'example-{name}','version':version} for name in ['core','macros','app']]
        argv = [sys.executable,str(ADAPTER),str(registry),json.dumps(inventory),cargo,
                'package','--locked','--offline','--allow-dirty','--registry','crates-io']

        def run(success=True):
            result = subprocess.run(argv,cwd=project,env=env,capture_output=True,text=True,timeout=180)
            assert (result.returncode == 0) == success, (result.stdout,result.stderr)
            assert files(registry) == captured and config.read_bytes() == configuration
            assert not list(home.glob('oyzu-package-*')), 'private registry survived invocation'
            return result

        run()
        output = project/'.oyzu-build/target/package'
        archives = sorted(output.glob('*.crate'))
        assert len(archives) == 3
        digests = {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in archives}
        for path in archives:
            with tarfile.open(path) as archive:
                prefix = path.name.removesuffix('.crate')+'/'
                manifest = tomllib.loads(archive.extractfile(prefix+'Cargo.toml').read().decode())
                assert manifest['package']['version'] == version
                assert not any('.oyzu-build/' in p for p in archive.getnames())
                for name, dependency in manifest.get('dependencies',{}).items():
                    assert dependency['version'] == ('=1.0.0' if name == 'fixture-dep' else '='+version)
                    assert 'path' not in dependency
                lock = tomllib.loads(archive.extractfile(prefix+'Cargo.lock').read().decode())
                if manifest['package']['name'] != 'example-macros':
                    assert any(p['name']=='fixture-dep' and p['version']=='1.0.0' and p.get('checksum') for p in lock['package'])
        assert files(project) == before
        run()
        assert digests == {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in archives}
        adapter = runpy.run_path(str(ADAPTER))
        index = output/'tmp-registry/index/ex/am/example-app'
        original_index = index.read_bytes()
        entry = json.loads(original_index)
        entry['cksum'] = '0'*64
        index.write_text(json.dumps(entry)+'\n')
        try:
            adapter['promote'](output.parent, base/'rejected-registry', inventory[-1])
        except ValueError as error:
            assert 'digest' in str(error)
        else:
            raise AssertionError('tampered native index admitted')
        assert not (base/'rejected-registry').exists()
        index.write_bytes(original_index)
        manifest = project/'core/Cargo.toml'
        original_manifest = manifest.read_bytes()
        manifest.write_text(manifest.read_text().replace('[package]', '[package]\nexclude=["message.txt"]'))
        native(project,'build','--workspace','--locked','--offline')
        run(False)  # Archive verification must rebuild from the incomplete crate.
        manifest.write_bytes(original_manifest)
        source.write_text(source.read_text()+'\ncompile_error!("native packaging failure");\n')
        run(False)
    print('Native Cargo mixed packaging passed: verified snapshot crates, registry dependencies, unchanged inputs/config, repeatability and failure cleanup')


if __name__ == '__main__':
    main()
