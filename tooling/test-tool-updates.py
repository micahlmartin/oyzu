"""Exercise explicit Node updates using projects provisioned by test-tool-management.py."""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--workspace", required=True, type=Path)
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    root = args.workspace.resolve(strict=True)
    project = root / "22.15.0"
    other = root / "22.14.0"
    store = root / "store"
    config = project / "oyzu.toml"
    original_config = config.read_text(encoding="utf-8")
    old_lock = (project / "oyzu.lock").read_bytes()
    other_lock = (other / "oyzu.lock").read_bytes()

    def run(project, *command, expected=0):
        result = subprocess.run([str(cli), "-C", str(project), *command], capture_output=True, text=True, timeout=300)
        assert result.returncode == expected, (command, result.returncode, result.stdout, result.stderr)
        return result.stdout.strip(), result.stderr

    # A TOML edit must not silently switch frozen selection or rewrite its lock.
    config.write_text(original_config.replace('"22.15.0"', '"22.14.0"'), encoding="utf-8")
    _, error = run(project, "install", "--store", str(store), expected=2)
    assert "TOOL_LOCK_STALE" in error, error
    run(project, "exec", "--store", str(store), "--", "node", "--version", expected=2)
    assert (project / "oyzu.lock").read_bytes() == old_lock
    run(project, "install", "--update", "go", "--store", str(store), expected=2)
    run(project, "install", "--update", "--frozen", "--store", str(store), expected=2)
    run(project, "install", "--update", "--offline", "--store", str(store), expected=2)
    assert (project / "oyzu.lock").read_bytes() == old_lock

    output, _ = run(project, "install", "--update", "node", "--store", str(store))
    assert "22.15.0 -> 22.14.0" in output, output
    assert run(project, "exec", "--store", str(store), "--", "node", "--version")[0] == "v22.14.0"
    assert (project / "oyzu.lock").read_bytes() != old_lock
    assert (other / "oyzu.lock").read_bytes() == other_lock
    assert run(other, "exec", "--store", str(store), "--", "node", "--version")[0] == "v22.14.0"

    # Restore the original requirement through the product, not by replacing its lock.
    config.write_text(original_config, encoding="utf-8")
    output, _ = run(project, "install", "--update", "core:node", "--store", str(store))
    assert "22.14.0 -> 22.15.0" in output, output
    assert run(project, "exec", "--store", str(store), "--", "node", "--version")[0] == "v22.15.0"
    before = (project / "oyzu.lock").read_bytes()
    run(project, "install", "--update", "--store", str(store))
    assert (project / "oyzu.lock").read_bytes() == before
    assert (other / "oyzu.lock").read_bytes() == other_lock
    print(json.dumps({"verified": ["stale config rejects ordinary install and exec", "named update switches the actual Node executable", "another project's lock and selection are preserved", "canonical-name update restores the original version", "bare update is byte-identical when resolution is unchanged", "unsupported names and conflicting update flags fail without lock changes"]}, indent=2))


if __name__ == "__main__":
    main()
