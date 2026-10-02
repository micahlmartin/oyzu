"""Exercise explicit development commands with a real frozen Node selection."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True,
                        help="Workspace retained by test-tool-management.py")
    args = parser.parse_args()
    root = args.workspace.resolve(strict=True)
    project = root / "22.15.0"
    store = root / "store"
    cli = args.cli.resolve(strict=True)
    lock = (project / "oyzu.lock").read_bytes()

    def run(arguments, expected=0):
        result = subprocess.run([str(cli), "-C", str(project)] + arguments,
                                capture_output=True, text=True, timeout=180)
        assert result.returncode == expected, (arguments, result.stdout, result.stderr)
        return result.stdout.strip()

    selected = Path(run(["which", "--store", str(store), "node"]))
    prefix = ["exec", "--store", str(store), "--"]
    script = "import json,os,subprocess,sys; print(json.dumps([subprocess.check_output(['node','--version'],text=True).strip(),os.getcwd(),os.environ['APP_ACCEPTANCE'],sys.argv[1:]]))"
    literal = ["a b", 'a"b', "&|<>^()%!"]
    observed = json.loads(run(prefix + [sys.executable, "-c", script] + literal))
    assert observed == ["v22.15.0", str(project), "from-toml", literal], observed
    run(prefix + [sys.executable, "-c", "import sys; sys.exit(7)"], expected=7)
    relative = os.path.relpath(selected, project)
    assert run(prefix + [relative, "--version"]) == "v22.15.0"
    # This interpreter is demonstrably installed; its bare name must still not
    # acquire permission merely because it exists on the host.
    run(prefix + [Path(sys.executable).name, "--version"], expected=2)
    run(prefix + ["./missing-executable"], expected=2)
    run(prefix + ["./"], expected=2)
    assert (project / "oyzu.lock").read_bytes() == lock
    print(json.dumps({"verified": ["explicit interpreter inherits locked Node, cwd, literal arguments and TOML environment",
                                  "explicit relative executable resolves against -C", "child exit status preserved",
                                  "unknown bare and unavailable paths fail", "lock remains unchanged"]}, indent=2))


if __name__ == "__main__":
    main()
