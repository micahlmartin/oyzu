"""Exercise a real Node/Go project and an update that preserves the other root."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib


def check_shell(cli, project, store, shell):
    environment = dict(os.environ, FRONTEND=str(cli), STORE=str(store))
    if shell == "pwsh":
        script = '$ErrorActionPreference = "Stop"; & $env:FRONTEND activate pwsh --store $env:STORE | Out-String | Invoke-Expression; if ($env:OYZU_TOOL_STATUS -ne "ready") { throw "selection not ready" }; node --version; go version; oyzu deactivate'
        invocation = [shutil.which(shell), "-NoProfile", "-NonInteractive", "-Command", script]
    else:
        script = f'set -e; eval "$("$FRONTEND" activate {shell} --store "$STORE")"; test "$OYZU_TOOL_STATUS" = ready; node --version; go version; oyzu deactivate'
        invocation = [shutil.which(shell), "-c", script]
    result = subprocess.run(invocation, cwd=project, env=environment, capture_output=True, text=True, timeout=900)
    assert result.returncode == 0, (result.stdout, result.stderr)
    lines = result.stdout.strip().splitlines()
    assert lines[0] == "v22.14.0" and lines[1].startswith("go version go1.24.13 "), lines


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--store", type=Path)
    parser.add_argument("--offline-check", action="store_true")
    parser.add_argument("--shell", choices=["bash", "zsh", "pwsh"])
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    root = args.workspace.resolve()
    store = (args.store or root / "store").resolve()
    project = root / "project"

    def configure(node, go="1.24.13"):
        (project / "oyzu.toml").write_text(f'[tools]\nnode = "{node}"\ngo = "{go}"\n[env]\nAPP_ACCEPTANCE = "mixed"\n', encoding="utf-8")

    def run(arguments, expected=0):
        result = subprocess.run([str(cli), "-C", str(project)] + arguments,
                                capture_output=True, text=True, timeout=900)
        assert result.returncode == expected, (arguments, result.stdout, result.stderr)
        return result.stdout.strip()

    install = ["install", "--store", str(store)]
    execute = ["exec", "--store", str(store), "--"]
    if not args.offline_check:
        project.mkdir(parents=True)
        configure("22.15.0")
        print("Installing mixed Node/Go selection", file=sys.stderr, flush=True)
        run(install)
        before = tomllib.loads((project / "oyzu.lock").read_text(encoding="utf-8"))
        assert len(before["tool"]) == 2
        go_before = next(record for record in before["tool"] if record["id"] == "core:go")
        assert run(execute + ["node", "--version"]) == "v22.15.0"
        # A changed unselected request must not be silently adopted by --update.
        lock = (project / "oyzu.lock").read_bytes()
        configure("22.14.0", "1.24.12")
        run(install + ["--update", "node"], expected=2)
        assert (project / "oyzu.lock").read_bytes() == lock
        configure("22.14.0")
        run(install, expected=2)
        print("Updating only Node; preserving Go", file=sys.stderr, flush=True)
        run(install + ["--update", "node"])
        after = tomllib.loads((project / "oyzu.lock").read_text(encoding="utf-8"))
        assert next(record for record in after["tool"] if record["id"] == "core:go") == go_before
    lock = (project / "oyzu.lock").read_bytes()
    print("Executing both locked tools and the composed environment", file=sys.stderr, flush=True)
    run(install + ["--frozen", "--offline"])
    assert run(execute + ["node", "--version"]) == "v22.14.0"
    assert run(execute + ["go", "version"]).startswith("go version go1.24.13 ")
    go_path = Path(run(["which", "--store", str(store), "go"]))
    node_path = Path(run(["which", "--store", str(store), "node"]))
    script = "const c=require('child_process'); console.log(JSON.stringify([process.execPath,process.env.APP_ACCEPTANCE,c.execFileSync('go',['env','GOROOT'],{encoding:'utf8'}).trim()]))"
    observed = json.loads(run(execute + ["node", "-e", script]))
    assert Path(observed[0]).resolve() == node_path.resolve()
    assert observed[1] == "mixed"
    assert Path(observed[2]).resolve() == go_path.parent.parent.resolve()
    environment = json.loads(run(["env", "--store", str(store), "--json"]))
    assert set(environment["tools"]) == {"core:node", "core:go"}
    assert environment["environment"]["APP_ACCEPTANCE"] == "<redacted>"
    assert (project / "oyzu.lock").read_bytes() == lock
    if args.shell:
        check_shell(cli, project, store, args.shell)
    print(json.dumps({"verified": ["mixed Node/Go frozen selection and executable discovery",
                                  "Node child uses the selected Go environment", "redacted multi-tool inspection", "unchanged frozen lock"] +
                     ([] if args.offline_check else ["Node-only update preserves the exact Go record", "unselected changed requirement fails without lock mutation"]) +
                     ([f"{args.shell}: both selected tools execute through shell activation"] if args.shell else [])}, indent=2))


if __name__ == "__main__":
    main()
