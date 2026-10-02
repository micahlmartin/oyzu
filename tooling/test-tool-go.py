"""Real Go acquisition, frozen replay, compiler execution and shell shim acceptance."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--offline-check", action="store_true")
    parser.add_argument("--restore-cached", action="store_true")
    parser.add_argument("--shell", choices=["bash", "zsh", "pwsh"])
    args = parser.parse_args()
    if args.restore_cached and not args.offline_check:
        parser.error("--restore-cached requires --offline-check")
    cli = args.cli.resolve(strict=True)
    root = args.workspace.resolve()
    project = root / "project"
    store = root / "store"
    if not args.offline_check:
        project.mkdir(parents=True)
        (project / "oyzu.toml").write_text('[tools]\ngo = "1.24.13"\n[env]\nAPP_ACCEPTANCE = "from-toml"\n', encoding="utf-8")
        (project / "hello.go").write_text('package main\nimport ("fmt"; "os")\nfunc main() { fmt.Printf("%s|%s", os.Getenv("APP_ACCEPTANCE"), os.Args[1]) }\n', encoding="utf-8")

    def run(arguments, expected=0):
        result = subprocess.run([str(cli), "-C", str(project)] + arguments,
                                capture_output=True, text=True, timeout=600)
        assert result.returncode == expected, (arguments, result.stdout, result.stderr)
        return result.stdout.strip()

    if args.restore_cached:
        (store / "installs").rename(root / "installs-before-restore")
    if not args.offline_check:
        run(["install", "--store", str(store)])
    lock = (project / "oyzu.lock").read_bytes()
    run(["install", "--store", str(store), "--frozen", "--offline"])
    executable = Path(run(["which", "--store", str(store), "go"]))
    command = ["exec", "--store", str(store), "--", "go"]
    assert run(command + ["version"]).startswith("go version go1.24.13 ")
    assert Path(run(command + ["env", "GOROOT"])).resolve() == executable.parent.parent.resolve()
    assert run(command + ["env", "GOTOOLCHAIN"]) == "local"
    assert run(command + ["run", "hello.go", "a b &|<>"]) == "from-toml|a b &|<>"
    assert (project / "oyzu.lock").read_bytes() == lock
    verified = ["real Go 1.24.13 install/frozen reuse and which/exec agreement",
                "locked GOROOT and local toolchain; actual compiler runs a program with literal args/TOML env",
                "unchanged lock"]
    if args.shell:
        environment = dict(os.environ, FRONTEND=str(cli), STORE=str(store))
        if args.shell == "pwsh":
            script = '$ErrorActionPreference = "Stop"; & $env:FRONTEND activate pwsh --store $env:STORE | Out-String | Invoke-Expression; if ($env:OYZU_TOOL_STATUS -ne "ready" -or (Get-Command go).Source -notlike "*shims*go.exe") { throw "Go shim not active" }; go version; if ($LASTEXITCODE -ne 0) { throw "Go failed" }; oyzu deactivate'
            invocation = [shutil.which(args.shell), "-NoProfile", "-NonInteractive", "-Command", script]
        else:
            script = f'set -e; eval "$("$FRONTEND" activate {args.shell} --store "$STORE")"; test "$OYZU_TOOL_STATUS" = ready; case "$(command -v go)" in *shims*/go) ;; *) exit 1;; esac; go version; oyzu deactivate'
            invocation = [shutil.which(args.shell), "-c", script]
        result = subprocess.run(invocation, cwd=project, env=environment, capture_output=True, text=True, timeout=600)
        assert result.returncode == 0 and result.stdout.startswith("go version go1.24.13 "), (result.stdout, result.stderr)
        verified.append(f"{args.shell}: native Go shim activates and executes")
    if args.restore_cached:
        verified.append("missing installation restored from verified cached archive")
    print(json.dumps({"verified": verified, "scope": "standalone single Go tool; no mixed-tool or managed qualification"}, indent=2))


if __name__ == "__main__":
    main()
