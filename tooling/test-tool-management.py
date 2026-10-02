"""Run the opt-in real Node install/exec user flow; downloads public Node archives."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from contextlib import nullcontext


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--workspace", type=Path, help="Preserve projects for a later offline replay")
    parser.add_argument("--offline-check", action="store_true", help="Replay an already provisioned workspace without downloads")
    parser.add_argument("--restore-cached", action="store_true", help="With offline replay, move installed trees aside and restore from cached archives")
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    verified = []
    if args.offline_check and not args.workspace:
        parser.error("--offline-check requires --workspace")
    if args.restore_cached and not args.offline_check:
        parser.error("--restore-cached requires --offline-check")
    with (nullcontext(args.workspace) if args.workspace else tempfile.TemporaryDirectory(prefix="oyzu-node-acceptance-")) as temporary:
        root = Path(temporary).resolve()
        root.mkdir(parents=True, exist_ok=True)
        store = root / "store"
        if args.restore_cached:
            # Preserve the original proof state; no recursive deletion is needed.
            (store / "installs").rename(root / "installs-before-restore")
        projects = []
        for version in ("22.15.0", "22.14.0"):
            print(f"Checking Node {version} ({'offline replay' if args.offline_check else 'install and reuse'})", file=sys.stderr, flush=True)
            project = root / version
            if not args.offline_check:
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
            if not args.offline_check:
                run(["install", "--frozen", "--store", str(store)], expected=2)
                assert not (project / "oyzu.lock").exists()
                run(["install", "--store", str(store)])
            lock = (project / "oyzu.lock").read_bytes()
            if args.restore_cached:
                run(["install", "--frozen", "--offline", "--store", str(root / "empty-store")], expected=2)
            run(["install", "--frozen", "--store", str(store)] + (["--offline"] if args.offline_check else []))
            run(["install", "--store", str(store)])
            executable = Path(run(["which", "--store", str(store), "node"]))
            assert executable.is_file(), executable
            command = ["exec", "--store", str(store), "--", "node"]
            assert Path(run(command + ["-p", "process.execPath"])).resolve() == executable.resolve()
            assert run(command + ["--version"]) == "v" + version
            observed = json.loads(run(command + ["-e", "console.log(JSON.stringify([process.env.APP_ACCEPTANCE,process.cwd(),...process.argv.slice(1)]))", "a b", 'a"b', "&|<>^()%!"]))
            assert observed == ["from-toml", str(project), "a b", 'a"b', "&|<>^()%!"], observed
            run(command + ["-e", "process.exit(7)"], expected=7)
            assert (project / "oyzu.lock").read_bytes() == lock
            projects.append((project, version))
            verified.append(f"Node {version}: TOML, {'offline reuse' if args.offline_check else 'real install'}, frozen/repeat install, which/exec agreement, arguments, cwd, environment, exit status")
        for project, version in reversed(projects):
            result = subprocess.run([str(cli), "-C", str(project), "exec", "--store", str(store), "--", "node", "--version"], capture_output=True, text=True, check=True, timeout=120)
            assert result.stdout.strip() == "v" + version
        verified.append("two projects select their own locked Node versions from one store")
        if args.restore_cached:
            verified.append("absent installations restored from verified cached archives; empty cache rejected in offline mode")
    print(json.dumps({"verified": verified, "scope": "standalone Node development install/exec; no managed or shell qualification"}, indent=2))


if __name__ == "__main__":
    main()
