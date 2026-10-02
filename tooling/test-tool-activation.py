"""Exercise real upstream-generated Oyzu hooks, project switching and cleanup."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--workspace", required=True, type=Path)
    parser.add_argument("--shell", action="append", choices=["bash", "zsh", "pwsh"], required=True)
    args = parser.parse_args()
    root = args.workspace.resolve(strict=True)
    missing = root / "missing-selection"
    missing.mkdir(exist_ok=True)
    (missing / "oyzu.toml").write_text('[tools]\nnode = "22.15.0"\n', encoding="utf-8")
    script = r'''
set -e
export APP_ACCEPTANCE=before
ambient_node=$("$FRONTEND" -C "$WORKSPACE/22.15.0" which --store "$WORKSPACE/store" node)
export PATH="${ambient_node%/*}:$PATH"
test "$(node --version)" = v22.15.0
original_path=$PATH
cd "$WORKSPACE/22.15.0"
eval "$("$FRONTEND" activate "$SHELL_KIND" --store "$WORKSPACE/store")"
test "$OYZU_TOOL_STATUS" = ready
test "$(node --version)" = v22.15.0
case "$(command -v node)" in */shims/*/node) ;; *) exit 1;; esac
test "$(node -e 'console.log(process.argv[1])' 'a b')" = 'a b'
if node -e 'process.exit(7)'; then exit 1; else test "$?" = 7; fi
test "$APP_ACCEPTANCE" = from-toml
test -n "$OYZU_SHELL_SESSION"
test -z "$("$FRONTEND" hook-env -s "$SHELL_KIND")"
cd "$WORKSPACE/22.14.0"
test "$(node --version)" = v22.14.0
export APP_ACCEPTANCE=user-edited
cd "$WORKSPACE/missing-selection"
test "$OYZU_TOOL_STATUS" = unavailable
test "$APP_ACCEPTANCE" = user-edited
if node --version >"$WORKSPACE/missing-node-output" 2>&1; then exit 1; fi
cd "$WORKSPACE/22.15.0"
test "$(node --version)" = v22.15.0
oyzu deactivate
test "$PATH" = "$original_path"
test "$APP_ACCEPTANCE" = user-edited
test -z "${OYZU_SHELL_SESSION-}"
test -z "${OYZU_TOOL_STATUS-}"
test -z "${OYZU_SHELL-}"
printf 'activation passed\n'
'''
    verified = []
    powershell = r'''
$ErrorActionPreference = 'Stop'
function Assert($condition, $message) { if (-not $condition) { throw $message } }
$env:APP_ACCEPTANCE = 'before'
$ambientNode = & $env:FRONTEND -C "$env:WORKSPACE/22.15.0" which --store "$env:WORKSPACE/store" node
$env:PATH = (Split-Path -Parent $ambientNode) + [IO.Path]::PathSeparator + $env:PATH
Assert ((node --version) -eq 'v22.15.0') 'ambient Node setup'
$originalPath = $env:PATH
Set-Location "$env:WORKSPACE/22.15.0"
& $env:FRONTEND activate pwsh --store "$env:WORKSPACE/store" | Out-String | Invoke-Expression
Assert ($env:OYZU_TOOL_STATUS -eq 'ready') 'activation status'
Assert ((node --version) -eq 'v22.15.0') 'initial version'
Assert ((Get-Command node).Source -like '*shims*node.exe') 'native executable shim'
Assert ((node -e 'console.log(process.argv[1])' 'a b') -eq 'a b') 'literal arguments'
node -e 'process.exit(7)'
Assert ($LASTEXITCODE -eq 7) 'native exit status'
Assert ($env:APP_ACCEPTANCE -eq 'from-toml') 'initial environment'
Set-Location "$env:WORKSPACE/22.14.0"
Assert ((node --version) -eq 'v22.14.0') 'automatic switch'
$env:APP_ACCEPTANCE = 'user-edited'
Set-Location "$env:WORKSPACE/missing-selection"
Assert ($env:OYZU_TOOL_STATUS -eq 'unavailable') 'missing status'
Assert ($env:APP_ACCEPTANCE -eq 'user-edited') 'preserve edit'
node --version 2>$null
Assert ($LASTEXITCODE -eq 2) 'missing selection must not use system node'
Set-Location "$env:WORKSPACE/22.15.0"
Assert ((node --version) -eq 'v22.15.0') 'return version'
oyzu deactivate
Assert ($env:PATH -eq $originalPath) 'deactivation PATH'
Assert ($env:APP_ACCEPTANCE -eq 'user-edited') 'deactivation scalar'
Assert (-not $env:OYZU_SHELL_SESSION) 'session cleanup'
Assert (-not $env:OYZU_TOOL_STATUS) 'status cleanup'
Write-Output 'activation passed'
'''
    for shell in args.shell:
        executable = shutil.which(shell)
        assert executable, f"required shell unavailable: {shell}"
        environment = dict(os.environ, FRONTEND=str(args.cli.resolve(strict=True)), WORKSPACE=str(root), SHELL_KIND=shell)
        if shell == "pwsh":
            command = [executable, "-NoProfile", "-NonInteractive", "-Command", powershell]
        else:
            command = [executable, "--noprofile", "--norc", "-c", script] if shell == "bash" else [executable, "-f", "-c", script]
        result = subprocess.run(command, env=environment, capture_output=True, text=True, timeout=300)
        assert result.returncode == 0, (shell, result.returncode, result.stdout, result.stderr)
        assert result.stdout.strip() == "activation passed", result.stdout
        verified.append(f"{shell}: native shim selection/arguments/exit status, automatic cd switching, missing-selection failure without PATH fallback, user scalar edit preservation and deactivation")
    print(json.dumps({"verified": verified}, indent=2))


if __name__ == "__main__":
    main()
