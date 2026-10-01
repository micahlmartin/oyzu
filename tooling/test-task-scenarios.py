"""Black-box checks against the compiled CLI and real example sources.

These validate task behavior only. They do not claim completed build scenarios.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, required=True)
    args = parser.parse_args()
    cli = args.cli.resolve()
    if not cli.is_file():
        raise SystemExit(f"Missing compiled CLI: {cli}")
    passed = []

    def invoke(project, *arguments, env=None, status=0):
        result = subprocess.run([str(cli), "-C", str(project), "--json", *arguments], capture_output=True, text=True, env=env, timeout=60)
        if (result.returncode == 0) != (status == 0):
            raise AssertionError(f"{arguments}: exit {result.returncode}\n{result.stdout}\n{result.stderr}")
        return json.loads(result.stdout) if result.stdout.strip() else result.stderr

    with tempfile.TemporaryDirectory(prefix="oyzu task scenarios ") as tmp:
        root = Path(tmp)

        def copy(example, name):
            dest = root / name
            shutil.copytree(ROOT / "examples" / example, dest, ignore=shutil.ignore_patterns("node_modules", "dist", ".events", ".venv"))
            return dest

        node = copy("tasks/npm-scripts/project", "native scripts")
        listing = invoke(node, "run", "list")
        assert any(t["name"] == "foo" and t["argv"] == ["npm", "run", "foo"] for t in listing.values())
        result = invoke(node, "run", "foo")
        assert len(result) == 1 and result[0]["status"] == "succeeded"
        events = [line for line in result[0]["stdout"].splitlines() if line in {"prefoo", "foo", "postfoo"}]
        assert events == ["prefoo", "foo", "postfoo"]
        passed.append("EX-007 native npm lifecycle executes exactly once")

        for failure, expected in [(None, ["pre", "main", "post"]), ("pre", ["pre"]), ("main", ["pre", "main"]), ("post", ["pre", "main", "post"])]:
            project = copy("tasks/overrides-and-hooks/project", "hooks " + (failure or "success"))
            env = dict(os.environ)
            env.pop("FAILURE_POINT", None)
            if failure:
                env["FAILURE_POINT"] = failure
            outcomes = invoke(project, "run", "api:test", env=env, status=1 if failure else 0)
            assert (project / "api/.events/order.txt").read_text().splitlines() == expected
            assert outcomes[-1]["status"] == ("failed" if failure else "succeeded")
            passed.append("EX-009 hook sequence " + (failure or "success"))

        grouped = copy("tasks/grouped-targets/project", "groups")
        listing = invoke(grouped, "run", "list")
        assert "api:test" in listing and "web:test" in listing
        error = invoke(grouped, "run", "test", status=1)
        assert "ambiguous" in error
        passed.append("EX-008 qualified tasks and ambiguity")

        node_build = copy("builds/node-package/project", "native build")
        outcomes = invoke(node_build, "run", "build")
        assert outcomes[-1]["status"] == "succeeded"
        assert (node_build / "dist/greeting.mjs").read_bytes() == (node_build / "src/greeting.mjs").read_bytes()
        outcomes = invoke(node_build, "run", "test")
        assert outcomes[-1]["status"] == "succeeded"
        passed.append("EX-018 real native build and test task execution")

        if os.name == 'nt':
            launchers = root/'native launchers'
            launchers.mkdir()
            for suffix in ['cmd', 'bat']:
                tool = f'oyzu-native-{suffix}'
                (launchers/f'{tool}.{suffix}').write_text('@echo off\n@echo native-launcher-ok\n')
                (node_build/'oyzu.toml').write_text(f'[tasks.check]\nargv=["{tool}"]\n')
                env = dict(os.environ, PATH=str(launchers)+os.pathsep+os.environ['PATH'])
                outcomes = invoke(node_build, 'run', 'check', env=env)
                assert outcomes[-1]['stdout'].strip() == 'native-launcher-ok'
            passed.append('Windows development tasks resolve native cmd and bat launchers without ecosystem-specific names')
            # Ant also ships an OS/2 .cmd file. Follow the host PATHEXT order.
            (launchers/'oyzu-native.bat').write_text('@echo off\n@echo windows-batch\n')
            (launchers/'oyzu-native.cmd').write_text('@echo off\n@echo wrong-launcher\n')
            (node_build/'oyzu.toml').write_text('[tasks.check]\nargv=["oyzu-native"]\n')
            env['PATHEXT'] = '.COM;.EXE;.BAT;.CMD'
            outcomes = invoke(node_build, 'run', 'check', env=env)
            assert outcomes[-1]['stdout'].strip() == 'windows-batch'

    print(json.dumps({"verified": passed, "scope": "development task execution only; build integration pending"}, indent=2))


if __name__ == "__main__":
    main()
