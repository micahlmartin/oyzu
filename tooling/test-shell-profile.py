"""Install a real Node project, load an explicitly managed profile, and remove it."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--shell", action="append", choices=["bash", "zsh", "pwsh"], required=True)
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    verified = []
    with tempfile.TemporaryDirectory(prefix="oyzu-shell-profile-") as temporary:
        root = Path(temporary)
        (root / "oyzu.toml").write_text('[tools]\nnode = "22.15.0"\n', encoding="utf-8")

        def run(arguments, expected=0):
            result = subprocess.run([str(cli), "-C", str(root)] + arguments, capture_output=True, text=True, timeout=300)
            assert result.returncode == expected, (arguments, result.stdout, result.stderr)
            return result.stdout

        run(["install"])
        for shell in args.shell:
            profile = root / f"profile with spaces.{shell}"
            original = b"# User profile without a final newline"
            profile.write_bytes(original)
            command = ["shell", "install", shell, "--profile-path", str(profile)]
            run(command)
            installed = profile.read_bytes()
            run(command)
            assert profile.read_bytes() == installed
            # The block runs the real frontend; loading it must make the native
            # shim select and execute this project's locked Node.
            environment = dict(os.environ, PROFILE_FILE=str(profile))
            if shell == "pwsh":
                script = '. $env:PROFILE_FILE; if ($env:OYZU_TOOL_STATUS -ne "ready") { throw "profile not ready" }; node --version; oyzu deactivate'
                invocation = [shutil.which(shell), "-NoProfile", "-NonInteractive", "-Command", script]
            else:
                script = 'set -e; . "$PROFILE_FILE"; test "$OYZU_TOOL_STATUS" = ready; node --version; oyzu deactivate'
                invocation = [shutil.which(shell), "--noprofile", "--norc", "-c", script] if shell == "bash" else [shutil.which(shell), "-f", "-c", script]
            result = subprocess.run(invocation, cwd=root, env=environment, capture_output=True, text=True, timeout=300)
            assert result.returncode == 0, (shell, result.stdout, result.stderr)
            assert result.stdout.strip() == "v22.15.0", result.stdout
            # Preserve edits rather than replacing or removing user changes.
            profile.write_bytes(installed.replace(b"activate", b"activate --help", 1))
            altered = profile.read_bytes()
            run(command, expected=2)
            run(["shell", "remove", shell, "--profile-path", str(profile)], expected=2)
            assert profile.read_bytes() == altered
            profile.write_bytes(installed)
            run(["shell", "remove", shell, "--profile-path", str(profile)])
            assert profile.read_bytes() == original
            run(["shell", "remove", shell, "--profile-path", str(profile)])
            assert profile.read_bytes() == original
            verified.append(f"{shell}: managed profile loads real activation and locked Node; repeat install, edited-block refusal and byte-exact removal")
    print(json.dumps({"verified": verified}, indent=2))


if __name__ == "__main__":
    main()
