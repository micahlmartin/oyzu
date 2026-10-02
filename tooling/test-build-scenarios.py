"""Execute real captured-source builds through the compiled CLI and Docker.

These checks exercise build portions of EX-016/018/036/042. They do not mark
the complete authored scenario catalog as passing.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import time
import zipfile
import email
import xml.etree.ElementTree as ET

from jsonschema import Draft202012Validator, FormatChecker
from build_scenarios import ant, docker, go, gradle, helm, jest, materialization, maven, node, node_managers, node_preflight, node_workspaces, python_application, python_legacy, python_quality, rust, vitest

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def source_files(root):
    return {p.relative_to(root).as_posix(): digest(p) for p in root.rglob("*")
            if p.is_file() and not set(p.relative_to(root).parts) & {".oyzu", "dist"}}


def validate(bundle):
    records = {}
    for name in ["manifest", "plan", "envelope"]:
        path = bundle / f"{name}.json"
        if not path.exists():
            assert name == "plan"
            continue
        value = json.loads(path.read_text())
        schema = json.loads((ROOT / f"docs/contracts/v1alpha1/{name}.schema.json").read_text())
        Draft202012Validator(schema, format_checker=FormatChecker()).validate(value)
        records[name] = value
    for artifact in records["manifest"]["artifacts"] + records["manifest"]["reports"]:
        if "path" in artifact:
            assert digest(bundle / artifact["path"]) == artifact["digest"]
    return records["manifest"]


def application_coverage(project, manifest, module):
    report=next(r for r in manifest['reports'] if r['kind']=='coverage')
    assert report['summary']['covered']>0, 'executed application lines were not measured'
    document=ET.parse(project/'dist'/report['path'])
    classes=document.findall('.//class')
    assert classes and all('test_' not in c.attrib['filename'] for c in classes)
    sources=' '.join(n.text or '' for n in document.findall('.//source'))
    assert module in sources or any(module in c.attrib['filename'] for c in classes)
    assert any(int(line.attrib['hits'])>0 for line in document.findall('.//class/lines/line'))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--evidence-dir", type=Path, help="Retain generated bundles for CI review")
    args = parser.parse_args()
    cli = args.cli.resolve()
    verified = []
    evidence = args.evidence_dir.resolve() if args.evidence_dir else None
    if evidence:
        evidence.mkdir(parents=True, exist_ok=False)
    invocation = 0

    def invoke(root, *command, success=True):
        nonlocal invocation
        started = time.monotonic()
        print(f"[{root.name}] oyzu {' '.join(command)}", flush=True)
        result = subprocess.run([str(cli), "-C", str(root), *command], capture_output=True, text=True, timeout=900)
        print(f"[{root.name}] exit {result.returncode} after {time.monotonic()-started:.1f}s", flush=True)
        if evidence and command and command[0]=='build' and '--plan' not in command:
            invocation += 1
            destination = evidence / f'{invocation:02d}-{root.name}'
            destination.mkdir()
            if (root / 'dist').is_dir():
                shutil.copytree(root / 'dist', destination / 'dist')
            (destination / 'invocation.json').write_text(json.dumps({
                'command': ['oyzu', *command], 'expectedSuccess': success,
                'exitCode': result.returncode, 'stdout': result.stdout,
                'stderr': result.stderr,
            }, indent=2))
        if (result.returncode == 0) != success:
            logs = ""
            for p in (root / "dist/logs").glob("*"):
                logs += f"\n{p.name}:\n{p.read_text(errors='replace')[-12000:]}"
            raise AssertionError(f"{command}: {result.returncode}\n{result.stdout}\n{result.stderr}\n{logs}")
        return json.loads(result.stdout) if result.stdout.strip() else None

    with tempfile.TemporaryDirectory(prefix="oyzu-build-check-") as temporary:
        base = Path(temporary)
        node.verify_overrides(ROOT,base,invoke,validate,source_files,verified)
        node_preflight.verify(ROOT,base,invoke,validate,source_files,verified)
        node_managers.verify(ROOT,base,invoke,validate,source_files,verified)
        node_workspaces.verify(ROOT,base,invoke,validate,source_files,verified)
        jest.verify(ROOT,base,invoke,validate,source_files,verified)
        vitest.verify(ROOT,base,invoke,validate,source_files,verified)
        go.verify(ROOT,base,invoke,validate,source_files,verified)
        materialization.verify(ROOT,base,invoke,validate,source_files,verified)
        docker.verify(ROOT,base,invoke,validate,source_files,verified)
        maven.verify(ROOT,base,invoke,validate,source_files,verified)
        gradle.verify(ROOT,base,invoke,validate,source_files,verified)
        ant.verify(ROOT,base,invoke,validate,source_files,verified)
        helm.verify(ROOT,base,invoke,validate,source_files,verified)
        rust.verify(ROOT,base,invoke,validate,source_files,verified)
        python_application.verify(ROOT,base,invoke,validate,source_files,verified)
        python_legacy.verify(ROOT,base,invoke,validate,source_files,verified)
        python_quality.verify(ROOT,base,invoke,validate,source_files,verified)
        for example in ["node-package", "go-app"]:
            project = base / example
            shutil.copytree(ROOT / "examples/builds" / example / "project", project)
            before = source_files(project)
            first_plan = invoke(project, "build", "--plan")
            assert first_plan == invoke(project, "build", "--plan")
            result = invoke(project, "build")
            assert result["status"] == "succeeded"
            manifest = validate(project / "dist")
            assert manifest["planDigest"] == invoke(project, "inspect", "dist")["planDigest"]
            assert before == source_files(project), "build changed source files"
            assert len(manifest["artifacts"]) == 1
            artifact = manifest["artifacts"][0]
            assert "-dev.g" in artifact["version"]
            assert any(r["kind"] == "test" and r["summary"]["passed"] > 0 for r in manifest["reports"])
            assert any(r["kind"] == "coverage" and r["summary"]["total"] > 0 for r in manifest["reports"])
            path = project / "dist" / artifact["path"]
            if example == "node-package":
                with tarfile.open(path) as package:
                    metadata = json.load(package.extractfile("package/package.json"))
                    assert metadata["version"] == artifact["version"]
                    assert "package/dist/greeting.mjs" in package.getnames()
            else:
                result = subprocess.run(["docker", "run", "--rm", "--network=none", "--mount",
                                         f"type=bind,source={path},target=/app,readonly",
                                         "--entrypoint", "/app", "golang:1.24-bookworm"], capture_output=True, text=True)
                assert result.returncode == 0, (result.stdout,result.stderr)
                assert result.stdout.strip() == "Hello, Oyzu!"
                assert any(a["id"].endswith(":format-check") and a["status"] == "succeeded" for a in manifest["actions"])
            # Tampering is detected; a subsequent build retains the old bundle.
            path.write_bytes(path.read_bytes() + b"tampered")
            invoke(project,"inspect","dist",success=False)
            rebuilt = invoke(project,"build")
            assert rebuilt["artifacts"][0]["digest"] == artifact["digest"], "identical inputs changed artifact bytes"
            assert list((project / ".oyzu/history").iterdir())
            verified.append(f"{example}: plan, build, native tests, coverage, snapshot artifact, source isolation, bundle verification")

        project = base / "node-package"
        (project / "failure.test.mjs").write_text("import test from 'node:test'; test('intentional failure',()=>{throw new Error('expected')});\n")
        invoke(project, "build", success=False)
        manifest = validate(project / "dist")
        assert manifest["status"] == "failed" and not manifest["artifacts"]
        assert any(r["kind"] == "test" and r["summary"].get("failed",0) > 0 for r in manifest["reports"])
        assert next(a for a in manifest["actions"] if a["id"] == "project:package")["status"] == "blocked"
        invoke(project,"inspect","dist")
        verified.append("failed tests retain reports and block packaging")

        go_project = base / "go-app"
        (go_project / "main.go").write_text('package main\nimport "fmt"\nfunc greeting(name string) string { return "Hello, " + name + "!" }\nfunc main(){fmt.Println(greeting("Oyzu"))}\n')
        unformatted = source_files(go_project)
        invoke(go_project,"build",success=False)
        manifest = validate(go_project / "dist")
        assert next(a for a in manifest["actions"] if a["id"] == "project:format-check")["status"] == "failed"
        assert next(a for a in manifest["actions"] if a["id"] == "project:package")["status"] == "blocked"
        assert source_files(go_project) == unformatted
        verified.append("Go formatting failure blocks packaging without editing checkout")

        hooked = base / "hooks"
        shutil.copytree(ROOT / "examples/builds/node-package/project",hooked)
        (hooked / "hook.mjs").write_text("""import fs from 'node:fs';
import assert from 'node:assert/strict';
const phase=process.argv[2];
if(phase==='pre')fs.writeFileSync('.hook-order','pre');
else {assert.equal(fs.readFileSync('.hook-order','utf8'),'pre,build'); fs.writeFileSync('.hook-order','pre,build,post');}
""")
        with (hooked / "build.mjs").open("a") as build_script:
            build_script.write("\nconst hookFs=await import('node:fs');if(hookFs.readFileSync('.hook-order','utf8')!=='pre')throw new Error('missing pre hook');hookFs.appendFileSync('.hook-order',',build');\n")
        (hooked / "oyzu.toml").write_text('[tasks."project:pre_build"]\nargv=["node","hook.mjs","pre"]\n[tasks."project:post_build"]\nargv=["node","hook.mjs","post"]\n')
        invoke(hooked,"build")
        manifest = validate(hooked / "dist")
        ids = [a["id"] for a in manifest["actions"]]
        assert ids.index("project:pre_build") < ids.index("project:build") < ids.index("project:post_build")
        assert not (hooked / ".hook-order").exists()
        verified.append("build hooks execute around native build inside captured source")

        # A real build script checks the actual process environment and filesystem.
        isolated = base / "isolation"
        shutil.copytree(ROOT / "examples/builds/node-package/project",isolated)
        (isolated / "probe.mjs").write_text("""import fs from 'node:fs';
import assert from 'node:assert/strict';
import net from 'node:net';
assert.equal(process.env.OYZU_HOST_SECRET, undefined);
assert.equal(fs.existsSync('/var/run/docker.sock'),false);
assert.throws(()=>fs.writeFileSync('/forbidden-output','x'));
await new Promise((resolve,reject)=>{const s=net.connect({host:'1.1.1.1',port:443});s.setTimeout(2000);s.on('connect',()=>{s.destroy();reject(new Error('network escaped'))});s.on('error',resolve);s.on('timeout',()=>{s.destroy();resolve()})});
""")
        metadata = json.loads((isolated / "package.json").read_text())
        metadata["scripts"]["lint"] = "node probe.mjs"
        (isolated / "package.json").write_text(json.dumps(metadata))
        import os
        os.environ["OYZU_HOST_SECRET"] = "must-not-enter-build"
        invoke(isolated,"build")
        validate(isolated / "dist")
        verified.append("real container denies network, root writes, Docker socket and host environment inheritance")

        python_project = base / "python-api"
        shutil.copytree(ROOT / "examples/builds/python-api/project",python_project)
        metadata_path=python_project / "pyproject.toml"
        metadata_path.write_text(metadata_path.read_text().replace('dependencies = []','dependencies = ["packaging==24.2"]'))
        (python_project / "tests/test_dependencies.py").write_text('''import os
import socket
import packaging


def test_acquired_dependency_and_offline_boundary():
    assert packaging.__version__ == "24.2"
    assert not os.path.exists("/broker")
    assert "OYZU_HOST_SECRET" not in os.environ
    try:
        connection = socket.create_connection(("1.1.1.1", 443), timeout=1)
    except OSError:
        return
    connection.close()
    raise AssertionError("build network escaped")
''')
        python_before=source_files(python_project)
        invoke(python_project,"build")
        python_manifest=validate(python_project / "dist")
        assert source_files(python_project)==python_before
        for task in ['lint', 'format-check']:
            assert next(a for a in python_manifest['actions'] if a['id']==f'api:{task}')['status']=='succeeded'
        invoke(python_project,"inspect","dist")
        assert {a["name"] for a in python_manifest["artifacts"]}=={"wheel","sdist"}
        for artifact in python_manifest["artifacts"]:
            assert ".dev0+g" in artifact["version"]
            path=python_project / "dist" / artifact["path"]
            if artifact["name"]=="wheel":
                with zipfile.ZipFile(path) as archive:
                    member=next(n for n in archive.namelist() if n.endswith('.dist-info/METADATA'))
                    assert email.message_from_bytes(archive.read(member))["Version"]==artifact["version"]
                    assert 'api/__init__.py' in archive.namelist()
            else:
                with tarfile.open(path) as archive:
                    assert all(m.mtime==0 for m in archive.getmembers())
        assert next(r for r in python_manifest["reports"] if r["kind"]=="test")["summary"]["passed"]>=3
        assert next(r for r in python_manifest["reports"] if r["kind"]=="coverage")["summary"]["total"]>0
        application_coverage(python_project,python_manifest,'api')
        dependency_path=python_project / "dist/dependencies/api.json"
        dependency=json.loads(dependency_path.read_text())
        schema=json.loads((ROOT / "docs/contracts/v1alpha1/dependencies.schema.json").read_text())
        Draft202012Validator(schema).validate(dependency)
        assert any(p['name']=='packaging' and p['version']=='24.2' for p in dependency['packages'])
        assert any(p['name']=='setuptools' for p in dependency['packages'])
        assert all(p['digest'].startswith('sha256:') for p in dependency['packages'])
        rebuilt=invoke(python_project,"build")
        assert {a['name']:a['digest'] for a in rebuilt['artifacts']}=={a['name']:a['digest'] for a in python_manifest['artifacts']}
        verified.append("Python: broker acquisition, captured dependency closure, offline wheel/sdist builds, snapshot metadata, native tests/coverage and repeatable artifacts")

        with metadata_path.open('a') as metadata_file:
            metadata_file.write('\n[tool.coverage.report]\nfail_under=100\n')
        invoke(python_project,'build',success=False)
        insufficient=validate(python_project/'dist')
        assert insufficient['status']=='failed' and not insufficient['artifacts']
        assert next(r for r in insufficient['reports'] if r['kind']=='test')['summary']['failed']==0
        measured=next(r for r in insufficient['reports'] if r['kind']=='coverage')['summary']
        assert 0<measured['covered']<measured['total']
        assert next(a for a in insufficient['actions'] if a['id']=='api:package')['status']=='blocked'
        verified.append('Python coverage threshold blocks packaging despite passing tests and retains measured application coverage')

        uv_project=base / "python-uv-library"
        shutil.copytree(ROOT / "examples/builds/python-uv-library/project",uv_project)
        uv_before=source_files(uv_project)
        invoke(uv_project,"build")
        uv_manifest=validate(uv_project / "dist")
        assert source_files(uv_project)==uv_before
        for task in ['lint', 'format-check']:
            assert next(a for a in uv_manifest['actions'] if a['id']==f'project:{task}')['status']=='succeeded'
        uv_dependencies=json.loads((uv_project / "dist/dependencies/project.json").read_text())
        Draft202012Validator(schema).validate(uv_dependencies)
        assert uv_dependencies['manager']['id']=='uv'
        assert digest(uv_project/'uv.lock') in uv_dependencies['lockDigests']
        assert next(r for r in uv_manifest['reports'] if r['kind']=='test')['summary']['passed']>0
        assert {a['name'] for a in uv_manifest['artifacts']}=={'wheel','sdist'}
        application_coverage(uv_project,uv_manifest,'greeting')
        # A stale native lock must fail before the final execution plan exists.
        pyproject=uv_project / 'pyproject.toml'
        pyproject.write_text(pyproject.read_text().replace('pytest==8.3.5','pytest==8.3.4'))
        invoke(uv_project,'build',success=False)
        stale=validate(uv_project/'dist')
        assert stale['status']=='failed' and stale['planDigest'] is None
        verified.append('uv: native locked export, hash-checked wheel closure, native build/test wrappers and stale-lock rejection')

        poetry_project=base / 'python-poetry'
        shutil.copytree(ROOT / 'examples/builds/python-poetry/project',poetry_project)
        poetry_before=source_files(poetry_project)
        invoke(poetry_project,'build')
        poetry_manifest=validate(poetry_project/'dist')
        assert source_files(poetry_project)==poetry_before
        for task in ['lint', 'format-check']:
            assert next(a for a in poetry_manifest['actions'] if a['id']==f'project:{task}')['status']=='succeeded'
        poetry_dependencies=json.loads((poetry_project/'dist/dependencies/project.json').read_text())
        Draft202012Validator(schema).validate(poetry_dependencies)
        assert poetry_dependencies['manager']['id']=='poetry'
        assert digest(poetry_project/'poetry.lock') in poetry_dependencies['lockDigests']
        assert any(p['name']=='poetry-core' and p['version']=='2.2.1' for p in poetry_dependencies['packages'])
        assert next(r for r in poetry_manifest['reports'] if r['kind']=='test')['summary']['passed']>0
        assert {a['name'] for a in poetry_manifest['artifacts']}=={'wheel','sdist'}
        application_coverage(poetry_project,poetry_manifest,'greeting')
        invoke(poetry_project,'inspect','dist')
        pyproject=poetry_project/'pyproject.toml'
        pyproject.write_text(pyproject.read_text().replace('dependencies = []','dependencies = ["packaging==24.2"]'))
        invoke(poetry_project,'build',success=False)
        stale=validate(poetry_project/'dist')
        assert stale['status']=='failed' and stale['planDigest'] is None
        assert 'stale' in stale['diagnostics'][0]['message'].lower()
        verified.append('Poetry: native lock freshness/export, captured poetry-core backend, snapshot wheel/sdist and unittest results through pytest')

    summary={"verified":verified,"scope":"initial Node/npm, Go, Python manager, local Cargo workspace, Ant and local Helm chart builds; full builder catalog remains pending"}
    if evidence:
        (evidence/'summary.json').write_text(json.dumps(summary,indent=2))
    print(json.dumps(summary,indent=2))


if __name__ == "__main__":
    main()
