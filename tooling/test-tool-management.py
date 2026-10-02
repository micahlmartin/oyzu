"""Run the opt-in real Node install/exec user flow; downloads public Node archives."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    verified = []
    with tempfile.TemporaryDirectory(prefix="oyzu-node-acceptance-") as temporary:
        root = Path(temporary).resolve()
        store = root / "store"
        projects = []
        for version in ("22.15.0", "22.14.0"):
            project = root / version
            project.mkdir()
            (project / "oyzu.toml").write_text(
                f'[tools]\nnode = "{version}"\n[env]\nAPP_ACCEPTANCE = "from-toml"\n',
                encoding="utf-8",
            )
            base = [str(cli), "-C", str(project)]
            def run(command, expected=0):
                result = subprocess.run(base + command, capture_output=True, text=True, timeout=300)
                if result.returncode != expected:
                    raise RuntimeError(f"{command}: exit {result.returncode}\n{result.stdout}\n{result.stderr}")
                return result.stdout.strip()
            run(["install", "--store", str(store)])
            lock = (project / "oyzu.lock").read_bytes()
            command = ["exec", "--store", str(store), "--", "node"]
            assert run(command + ["--version"]) == "v" + version
            observed = json.loads(run(command + ["-e", "console.log(JSON.stringify([process.env.APP_ACCEPTANCE,process.cwd(),...process.argv.slice(1)]))", "a b", 'a"b', "&|<>^()%!"]))
            assert observed == ["from-toml", str(project), "a b", 'a"b', "&|<>^()%!"], observed
            run(command + ["-e", "process.exit(7)"], expected=7)
            assert (project / "oyzu.lock").read_bytes() == lock
            projects.append((project, version))
            verified.append(f"Node {version}: TOML, real install, frozen exec, arguments, cwd, environment, exit status")
        for project, version in reversed(projects):
            result = subprocess.run([str(cli), "-C", str(project), "exec", "--store", str(store), "--", "node", "--version"], capture_output=True, text=True, check=True, timeout=120)
            assert result.stdout.strip() == "v" + version
        verified.append("two projects select their own locked Node versions from one store")
    print(json.dumps({"verified": verified, "scope": "standalone Node development install/exec; no managed or shell qualification"}, indent=2))


if __name__ == "__main__":
    main()
