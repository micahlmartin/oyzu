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

from jsonschema import Draft202012Validator, FormatChecker

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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    verified = []

    def invoke(root, *command, success=True):
        result = subprocess.run([str(cli), "-C", str(root), *command], capture_output=True, text=True, timeout=900)
        if (result.returncode == 0) != success:
            logs = ""
            for p in (root / "dist/logs").glob("*"):
                logs += f"\n{p.name}:\n{p.read_text(errors='replace')[-12000:]}"
            raise AssertionError(f"{command}: {result.returncode}\n{result.stdout}\n{result.stderr}\n{logs}")
        return json.loads(result.stdout) if result.stdout.strip() else None

    with tempfile.TemporaryDirectory(prefix="oyzu-build-check-") as temporary:
        base = Path(temporary)
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
                                         "--entrypoint", "/app", "golang:1.24-bookworm"], capture_output=True, text=True, check=True)
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

    print(json.dumps({"verified":verified,"scope":"initial Node/npm and Go builds; full builder catalog remains pending"},indent=2))


if __name__ == "__main__":
    main()
