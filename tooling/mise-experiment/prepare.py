#!/usr/bin/env python3
"""Prepare an external pinned checkout; never build or execute a mise binary."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

PIN = "da0db43e9398b46bafa95232014708a51120e731"
HERE = Path(__file__).resolve().parent


def run(*args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--patch", action="store_true")
    args = parser.parse_args()
    source = args.source.resolve()
    repo = HERE.parents[1]
    if source == repo or repo in source.parents:
        parser.error("upstream checkout must be outside the Oyzu repository")
    if not source.exists():
        run("git", "init", str(source))
        run("git", "-C", str(source), "config", "core.autocrlf", "false")
        run("git", "-C", str(source), "remote", "add", "origin", "https://github.com/jdx/mise.git")
        run("git", "-C", str(source), "fetch", "--depth=1", "origin", PIN)
        run("git", "-C", str(source), "checkout", "--detach", "FETCH_HEAD")
    if run("git", "rev-parse", "HEAD", cwd=source) != PIN:
        parser.error("source HEAD does not match audited pin")
    # include_str! embeds these bytes unchanged. A Windows checkout's automatic
    # CRLF conversion breaks Bash activation when the same source builds on Linux.
    for name in run("git", "ls-files", "*.sh", cwd=source).splitlines():
        original = subprocess.check_output(["git", "show", f"HEAD:{name}"], cwd=source)
        path = source / name
        if path.read_bytes().replace(b"\r\n", b"\n") == original:
            path.write_bytes(original)
    license_text = (source / "LICENSE").read_text()
    if "MIT License" not in license_text or "Copyright (c) 2025 Jeff Dickey" not in license_text:
        parser.error("unexpected upstream license; review before proceeding")
    # The example uses the upstream workspace's exact dependency lock and only
    # links its library. It is an independently authored alternate frontend.
    examples = source / "examples"
    examples.mkdir(exist_ok=True)
    shutil.copyfile(HERE / "spike.rs", examples / "oyzu-mise-spike.rs")
    shutil.copyfile(HERE / "api-probe.rs", examples / "oyzu-api-probe.rs")
    if args.patch:
        patch = HERE / "embedding.patch"
        check = subprocess.run(["git", "apply", "--reverse", "--check", str(patch)], cwd=source, capture_output=True)
        if check.returncode:
            subprocess.run(["git", "apply", "--check", str(patch)], cwd=source, check=True)
            subprocess.run(["git", "apply", str(patch)], cwd=source, check=True)
    record = {
        "upstream": PIN,
        "license_sha256": hashlib.sha256(subprocess.check_output(["git", "show", "HEAD:LICENSE"], cwd=source)).hexdigest(),
        "lock_sha256": hashlib.sha256(subprocess.check_output(["git", "show", "HEAD:Cargo.lock"], cwd=source)).hexdigest(),
        "checkout_license_sha256": hashlib.sha256((source / "LICENSE").read_bytes()).hexdigest(),
        "checkout_lock_sha256": hashlib.sha256((source / "Cargo.lock").read_bytes()).hexdigest(),
        "patched": args.patch,
    }
    print(json.dumps(record, indent=2))


if __name__ == "__main__":
    main()
