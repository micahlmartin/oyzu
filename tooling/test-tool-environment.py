"""Apply mise-rendered Oyzu environment in real shells against installed Node."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--workspace", required=True, type=Path)
    parser.add_argument("--shell", action="append", choices=["bash", "zsh", "pwsh"], required=True)
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    root = args.workspace.resolve(strict=True)
    project = root / "22.15.0"
    config = project / "oyzu.toml"
    original = config.read_bytes()
    lock = (project / "oyzu.lock").read_bytes()
    literal = "literal 'quotes' \"double\" $HOME $(touch SHOULD_NOT_EXIST) `echo nope` ; & | < > !"
    verified = []
    try:
        # Appending under the existing [env] table changes environment, not tool identity.
        config.write_bytes(original + ("\nAPP_LITERAL = " + json.dumps(literal) + "\n").encode())
        base = [str(cli), "-C", str(project), "env", "--store", str(root / "store")]
        inspection = subprocess.run(base + ["--json"], check=True, capture_output=True, text=True, timeout=120)
        document = json.loads(inspection.stdout)
        assert document["environment"]["APP_LITERAL"] == "<redacted>"
        assert literal not in inspection.stdout
        node_code = "console.log(JSON.stringify([process.version,process.env.APP_LITERAL,process.env.APP_ACCEPTANCE,process.execPath]))"
        direct = subprocess.run([str(cli), "-C", str(project), "exec", "--store", str(root / "store"), "--", "node", "-e", node_code], check=True, capture_output=True, text=True, timeout=120)
        expected = json.loads(direct.stdout)
        assert expected[:3] == ["v22.15.0", literal, "from-toml"], expected
        for shell in args.shell:
            executable = shutil.which(shell)
            assert executable, f"required shell unavailable: {shell}"
            output = subprocess.run(base + ["--shell", shell], check=True, capture_output=True, text=True, timeout=120)
            script = root / ("environment." + ("ps1" if shell == "pwsh" else shell))
            if shell == "pwsh":
                script.write_text(output.stdout + "\nnode -e '" + node_code + "'\nexit $LASTEXITCODE\n", encoding="utf-8")
                command = [executable, "-NoProfile", "-NonInteractive", "-File", str(script)]
            else:
                script.write_text(output.stdout + "\nnode -e '" + node_code + "'\n", encoding="utf-8")
                command = [executable, "--noprofile", "--norc", str(script)] if shell == "bash" else [executable, "-f", str(script)]
            result = subprocess.run(command, cwd=project, check=True, capture_output=True, text=True, timeout=120)
            observed = json.loads(result.stdout)
            assert observed == expected, (shell, observed, expected)
            assert observed[:3] == ["v22.15.0", literal, "from-toml"], observed
            assert Path(observed[3]).resolve() == Path(document["executable"]).resolve()
            assert not (project / "SHOULD_NOT_EXIST").exists()
            verified.append(f"{shell}: upstream rendering applies literal config and launches locked Node")
        assert (project / "oyzu.lock").read_bytes() == lock
    finally:
        config.write_bytes(original)
    print(json.dumps({"verified": verified, "scope": "one-shot environment application, not automatic activation/deactivation"}, indent=2))


if __name__ == "__main__":
    main()
