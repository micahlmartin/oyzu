"""Read-only upstream observation. Never updates a pin or approves a release."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
FORK = "https://github.com/oyzuai/mise"
TESTED_BASE = "da0db43e9398b46bafa95232014708a51120e731"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("GitHub API redirects require manual review")


def github(path):
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "oyzu-mise-maintenance"}
    if token := os.environ.get("GH_TOKEN"):
        headers["Authorization"] = "Bearer " + token
    request = urllib.request.Request("https://api.github.com/" + path, headers=headers)
    with urllib.request.build_opener(NoRedirect).open(request, timeout=30) as response:
        raw = response.read(4 * 1024 * 1024 + 1)
    if len(raw) > 4 * 1024 * 1024:
        raise ValueError("GitHub response exceeds limit")
    return json.loads(raw)


def pinned_dependency(manifest, lock):
    dependency = manifest.get("dependencies", {}).get("mise")
    if dependency is None:
        return None
    if not isinstance(dependency, dict) or dependency.get("git") != FORK:
        raise ValueError("mise must use the public oyzuai/mise fork")
    rev = dependency.get("rev", "")
    if not re.fullmatch(r"[0-9a-f]{40}", rev) or "branch" in dependency or "tag" in dependency:
        raise ValueError("mise requires an immutable full commit rev")
    if dependency.get("default-features") is not False:
        raise ValueError("mise default features must be explicitly disabled")
    expected = f"git+{FORK}?rev={rev}#{rev}"
    packages = [p for p in lock.get("package", []) if p.get("name") == "mise"]
    if len(packages) != 1 or packages[0].get("source") != expected:
        raise ValueError("Cargo.lock does not match the mise fork pin")
    return rev


def observe(root, api=github):
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    pin = pinned_dependency(manifest, lock)
    release = api("repos/jdx/mise/releases/latest")
    if release.get("draft") or release.get("prerelease"):
        raise ValueError("latest stable endpoint returned a nonstable release")
    tag = release["tag_name"]
    from urllib.parse import quote
    commit = api("repos/jdx/mise/commits/" + quote(tag, safe=""))["sha"]
    fork = api("repos/oyzuai/mise")
    if fork.get("private") or fork.get("parent", {}).get("full_name") != "jdx/mise":
        raise ValueError("expected public fork of jdx/mise")
    fork_head = api("repos/oyzuai/mise/commits/" + quote(fork["default_branch"], safe=""))["sha"]
    for sha in [commit, fork_head]:
        if not re.fullmatch(r"[0-9a-f]{40}", sha):
            raise ValueError("invalid commit identity from GitHub")
    return {
        "schema": 1,
        "checked_at": datetime.now(timezone.utc).isoformat(),
        "upstream": "https://github.com/jdx/mise",
        "latest_stable": {"tag": tag, "commit": commit, "published_at": release["published_at"]},
        "fork": FORK,
        "fork_head": fork_head,
        "production_pin": pin,
        "experiment_base": TESTED_BASE,
        "status": "manual-triage-required" if pin else "production-integration-not-configured",
        "release_ready": False,
        "remaining_review": ["security advisories and exposure", "upstream delta and patches",
                             "dependency and license obligations", "platform qualification"],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        report = observe(ROOT)
    except Exception as error:
        # Do not include API bodies, URLs with credentials, or exception details.
        report = {"schema": 1, "checked_at": datetime.now(timezone.utc).isoformat(),
                  "status": "observation-failed", "error_type": type(error).__name__,
                  "release_ready": False}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 1 if report["status"] == "observation-failed" else 0


if __name__ == "__main__":
    raise SystemExit(main())
