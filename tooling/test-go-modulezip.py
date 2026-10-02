"""Native module zip consumption through a file proxy, with no project network."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--adapter', type=Path, required=True)
    parser.add_argument('--go', default='go')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='oyzu-go-modulezip-') as temporary:
        base = Path(temporary).resolve()
        env = dict(os.environ, GOENV='off', GOTOOLCHAIN='local', GOWORK='off', GOFLAGS='',
                   GOPROXY='off', GOSUMDB='off', GOPRIVATE='', GONOPROXY='', CGO_ENABLED='0',
                   GOMODCACHE=str(base/'modules'), GOCACHE=str(base/'cache'))
        project = base/'project'
        shutil.copytree(ROOT/'examples/builds/go-app/variants/library', project)

        def source():
            return {p.relative_to(project).as_posix():p.read_bytes() for p in project.rglob('*') if p.is_file()}

        def package(members, folder, fail=None, dependencies=None):
            metadata = base/'metadata.json'
            metadata.write_text(json.dumps({'modules':members,'binaries':[], 'moduleDependencies':dependencies or {}}))
            result = subprocess.run([str(args.adapter.resolve()),str(metadata),'0.0.0-dev.g0123456789ab',
                                     'go/app',str(folder)],cwd=project,env=env,capture_output=True,text=True,timeout=60)
            if fail:
                assert result.returncode != 0 and fail in result.stderr, (result.stdout,result.stderr)
                assert not (folder/'inventory.json').exists()
                return
            assert result.returncode==0, (result.stdout,result.stderr)
            inventory = json.loads((folder/'inventory.json').read_text())
            for item in inventory:
                with zipfile.ZipFile(folder/item['zip']) as archive:
                    assert archive.read(f"{item['path']}@{item['version']}/go.mod") == (folder/item['mod']).read_bytes()
                    assert all(n.startswith(f"{item['path']}@{item['version']}/") for n in archive.namelist())
                assert json.loads((folder/item['info']).read_text())['Version']==item['version']
                assert item['sum'].startswith('h1:') and item['goModSum'].startswith('h1:')
            return inventory

        def consume(folder, inventory, module, expression):
            proxy = folder/'proxy'
            for item in inventory:
                # These authored module paths have no uppercase escape sequences.
                destination = proxy/item['path']/'@v'
                destination.mkdir(parents=True)
                for extension in ['zip','mod','info']:
                    shutil.copyfile(folder/item[extension], destination/f"{item['version']}.{extension}")
            selected = next(item for item in inventory if item['path']==module)
            consumer = folder/'consumer'
            consumer.mkdir()
            (consumer/'go.mod').write_text(f'module example.com/consumer\n\ngo 1.22\n\nrequire {module} {selected["version"]}\n')
            (consumer/'consumer_test.go').write_text(f'package consumer\nimport ("testing"; lib "{module}")\nfunc TestPublished(t *testing.T) {{ if {expression} != 6 {{ t.Fatal("wrong published library") }} }}\n')
            result = subprocess.run([args.go,'test','-mod=mod','./...'],cwd=consumer,
                                    env={**env,'GOPROXY':proxy.as_uri(),'GOMODCACHE':str(folder/'consumer-modules')},capture_output=True,text=True,timeout=120)
            assert result.returncode==0, (result.stdout,result.stderr)
            sums = (consumer/'go.sum').read_text()
            assert selected['sum'] in sums and selected['goModSum'] in sums

        before = source()
        first = package(['.'],base/'first')
        assert first[0]['version']=='v2.0.0-dev.g0123456789ab'
        consume(base/'first',first,'example.com/oyzu/math/v2','lib.Double(3)')
        assert source()==before
        package(['.'],base/'second')
        for name in ['zip','mod','info']:
            assert (base/'first'/first[0][name]).read_bytes()==(base/'second'/first[0][name]).read_bytes()
        # Native zip validation rejects case-folded collisions instead of emitting
        # an archive that fails on another operating system.
        (project/'MATH.go').write_text('package math\n')
        if (project/'MATH.go').name in [p.name for p in project.iterdir()]:
            package(['.'],base/'collision',fail='case-insensitive')
        (project/'MATH.go').unlink()
        # Restore on case-insensitive hosts where writing MATH.go replaces math.go.
        for name, data in before.items():
            (project/name).write_bytes(data)
        workspace = base/'workspace'
        workspace.mkdir()
        shutil.move(str(project),str(workspace/'math'))
        project = workspace
        (project/'api').mkdir()
        (project/'api/go.mod').write_text('module example.com/oyzu/api\n\ngo 1.22\n\nrequire example.com/oyzu/math/v2 v2.0.0\nreplace example.com/oyzu/math/v2 => ../math\n')
        (project/'api/api.go').write_text('package api\nimport "example.com/oyzu/math/v2"\nfunc Value() int { return math.Double(3) }\n')
        (project/'go.work').write_text('go 1.22\nuse (\n./api\n./math\n)\n')
        before = source()
        modules = package(['api','math'],base/'workspace-output')
        assert b'replace' not in (base/'workspace-output'/modules[0]['mod']).read_bytes()
        assert modules[1]['version'] in (base/'workspace-output'/modules[0]['mod']).read_text()
        consume(base/'workspace-output',modules,'example.com/oyzu/api','lib.Value()')
        assert source()==before
        # Workspace imports need not already have a require entry in go.mod.
        (project/'api/go.mod').write_text('module example.com/oyzu/api\n\ngo 1.22\n')
        modules = package(['api','math'],base/'implicit-edge',dependencies={'example.com/oyzu/api':['example.com/oyzu/math/v2']})
        consume(base/'implicit-edge',modules,'example.com/oyzu/api','lib.Value()')
        with (project/'api/go.mod').open('a') as stream:
            stream.write('\nreplace example.com/unknown => ../math\n')
        package(['api','math'],base/'bad-local',fail='same identity')
        print('Go native module archives passed: independent consumer, /v2 identity, projected workspace dependency, h1 sums, byte repeatability, unchanged source and invalid replacement rejection')


if __name__ == '__main__':
    main()
