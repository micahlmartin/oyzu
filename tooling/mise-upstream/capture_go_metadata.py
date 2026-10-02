"""Capture public Go metadata for independent library replay qualification.

This is fixture provisioning, not the product broker, resolver or an install path.
It downloads no archives, executes nothing and grants no distribution approval.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import sys
import urllib.request

CATALOG = "https://go.dev/dl/?mode=json&include=all"
TARGETS = (("linux/amd64/gnu", "linux", "amd64", "tar.gz"),
           ("darwin/arm64/native", "darwin", "arm64", "tar.gz"),
           ("windows/amd64/msvc", "windows", "amd64", "zip"))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def fetch(url, limit):
    request = urllib.request.Request(url, headers={"User-Agent": "Oyzu-metadata-qualification"})
    with urllib.request.urlopen(request, timeout=30) as response:
        data = response.read(limit + 1)
    require(len(data) <= limit, "metadata response exceeds byte limit")
    return data


def capture(versions, acquire=fetch):
    """Collect exact requested releases; any missing/contradictory record fails."""
    require(0 < len(versions) <= 16 and len(set(versions)) == len(versions),
            "request one to sixteen unique exact versions")
    require(all(re.fullmatch(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", v)
                and len(v) <= 128 for v in versions), "versions must be canonical numeric triples")
    responses = {}

    def retain(url, limit):
        data = acquire(url, limit)
        require(len(data) <= limit, "metadata response exceeds byte limit")
        text = data.decode("utf-8", errors="strict")
        responses[url] = {"body_utf8": text, "sha256": digest(data), "size": len(data)}
        return text

    raw = retain(CATALOG, 16 * 1024 * 1024)
    records = json.loads(raw)
    require(isinstance(records, list) and len(records) <= 100_000, "invalid release catalog")
    expected = []
    for version in versions:
        matches = [record for record in records if record.get("version") == "go" + version]
        require(len(matches) == 1 and matches[0].get("stable") is True,
                "requested stable release missing or ambiguous")
        files = matches[0].get("files", [])
        require(isinstance(files, list) and len(files) <= 4096, "invalid file list")
        for target, os_name, arch, extension in TARGETS:
            name = f"go{version}.{os_name}-{arch}.{extension}"
            candidates = [item for item in files if item.get("filename") == name]
            require(len(candidates) == 1, "target archive missing or ambiguous")
            item = candidates[0]
            require(item.get("os") == os_name and item.get("arch") == arch
                    and item.get("version") == "go" + version and item.get("kind") == "archive",
                    "catalog target identity mismatch")
            require(type(item.get("size")) is int and 0 < item["size"] <= 2**64 - 1,
                    "invalid archive size")
            checksum = item.get("sha256", "")
            require(isinstance(checksum, str) and re.fullmatch(r"[a-fA-F0-9]{64}", checksum),
                    "invalid catalog checksum")
            url = "https://dl.google.com/go/" + name
            sidecar = retain(url + ".sha256", 128).strip()
            require(re.fullmatch(r"[a-fA-F0-9]{64}", sidecar)
                    and sidecar.lower() == checksum.lower(), "catalog/sidecar checksum disagreement")
            expected.append({"version": version, "target": target, "archive_url": url,
                             "declared_size": item["size"], "declared_sha256": "sha256:" + sidecar.lower(),
                             "catalog_sha256": responses[CATALOG]["sha256"]})
    return {"format": 1, "scope": "captured-public-go-metadata-replay-fixture",
            "captured_at": datetime.now(timezone.utc).isoformat(), "responses": responses,
            "expected": expected, "legal_approval": False, "release_ready": False,
            "limitations": ["HTTPS metadata consistency, not signature authentication",
                            "No archive acquisition, installation or target execution",
                            "Capture alone does not prove the Rust adapter passes replay"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.resolve().is_relative_to(Path(__file__).resolve().parents[2]),
            "retain captured metadata outside the repository")
    require(not args.output.exists(), "output already exists")
    result = capture(args.version)
    data = (json.dumps(result, indent=2) + "\n").encode("utf-8")
    output = args.output.open("xb")
    try:
        with output:
            output.write(data)
    except BaseException:
        args.output.unlink()
        raise
    print(json.dumps({"output": str(args.output), "fixture_sha256": digest(data),
                      "cases": len(result["expected"]), "responses": len(result["responses"]),
                      "legal_approval": False, "release_ready": False}, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, AttributeError, OSError) as error:
        print(f"Go metadata capture failed: {error}", file=sys.stderr)
        sys.exit(1)
