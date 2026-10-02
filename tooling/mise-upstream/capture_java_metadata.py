"""Capture public Temurin catalog fixtures; no artifacts, execution or approval."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import urllib.request

TARGETS = (("linux/amd64/gnu", "linux/x86_64", "tar.gz"),
           ("darwin/arm64/native", "macosx/aarch64", "tar.gz"),
           ("windows/amd64/msvc", "windows/x86_64", "zip"))
LIMIT = 16 * 1024 * 1024


def require(condition, message):
    if not condition:
        raise ValueError(message)


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "Oyzu-metadata-qualification"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read(LIMIT + 1)


def capture(acquire=fetch):
    responses, cases = {}, []
    for target, route, kind in TARGETS:
        url = "https://mise-java.jdx.dev/jvm/ga/" + route + ".json"
        raw = acquire(url)
        require(len(raw) <= LIMIT, "catalog exceeds byte limit")
        body = raw.decode("utf-8", errors="strict")
        rows = json.loads(body)
        require(isinstance(rows, list) and len(rows) <= 100_000
                and all(isinstance(row, dict) for row in rows), "invalid catalog")
        matches = [row for row in rows if row.get("vendor") == "temurin"
                   and row.get("version") == "21.0.6+7.0.LTS"
                   and row.get("image_type") == "jdk"
                   and row.get("jvm_impl") == "hotspot" and row.get("features") == []]
        require(len(matches) == 1, "exact JDK missing or ambiguous")
        row = matches[0]
        require(row.get("file_type") == kind, "wrong target archive kind")
        require(isinstance(row.get("checksum"), str)
                and re.fullmatch(r"sha256:[0-9a-f]{64}", row["checksum"]), "invalid checksum")
        suffix = {"linux/x86_64": "x64_linux", "macosx/aarch64": "aarch64_mac",
                  "windows/x86_64": "x64_windows"}[route]
        expected_url = ("https://github.com/adoptium/temurin21-binaries/releases/download/"
                        "jdk-21.0.6%2B7/OpenJDK21U-jdk_" + suffix + "_hotspot_21.0.6_7." + kind)
        require(row.get("url") == expected_url and row.get("java_version") == "21.0.6+7-LTS",
                "target or runtime identity mismatch")
        responses[url] = body
        cases.append({"target": target, "catalog_url": url,
                      "catalog_sha256": "sha256:" + hashlib.sha256(raw).hexdigest(),
                      "catalog_size": len(raw), "canonical_version": "temurin-" + row["version"],
                      "archive_url": expected_url, "declared_sha256": row["checksum"]})
    return {"format": 1, "captured_at": datetime.now(timezone.utc).isoformat(),
            "responses": responses, "cases": cases, "legal_approval": False, "release_ready": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.resolve().is_relative_to(Path(__file__).resolve().parents[2]),
            "retain original metadata outside the repository")
    require(not args.output.exists(), "output already exists")
    data = (json.dumps(capture(), indent=2) + "\n").encode("utf-8")
    require(len(data) <= 32 * 1024 * 1024, "fixture exceeds replay limit")
    with args.output.open("xb") as output:
        output.write(data)
    print(json.dumps({"fixture_sha256": "sha256:" + hashlib.sha256(data).hexdigest(),
                      "size": len(data), "cases": 3, "legal_approval": False, "release_ready": False}))


if __name__ == "__main__":
    main()
