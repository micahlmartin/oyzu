"""Retain bounded Python precompiled catalogs for replay; no install or approval."""
import argparse
import base64
from datetime import datetime, timezone
import gzip
import hashlib
import io
import json
from pathlib import Path
import urllib.request

TARGETS = (("linux/amd64/gnu", "x86_64-unknown-linux-gnu"),
           ("darwin/arm64/native", "aarch64-apple-darwin"),
           ("windows/amd64/msvc", "x86_64-pc-windows-msvc"))
LIMIT = 16 * 1024 * 1024


def digest(raw):
    return "sha256:" + hashlib.sha256(raw).hexdigest()


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "Oyzu-metadata-qualification"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read(LIMIT + 1)


def decode(raw, limit=LIMIT):
    if len(raw) > limit:
        raise ValueError("compressed catalog exceeds byte limit")
    with gzip.GzipFile(fileobj=io.BytesIO(raw)) as source:
        decoded = source.read(limit + 1)
    if len(decoded) > limit:
        raise ValueError("decoded catalog exceeds byte limit")
    decoded.decode("utf-8", errors="strict")
    return decoded


def capture(acquire=fetch):
    responses, cases = {}, []
    for target, triple in TARGETS:
        url = f"https://mise-versions.jdx.dev/tools/python-precompiled-{triple}.gz"
        raw = acquire(url)
        decoded = decode(raw)
        if not decoded.strip():
            raise ValueError("empty Python catalog")
        responses[url] = base64.b64encode(raw).decode("ascii")
        cases.append({"target": target, "catalog_url": url,
                      "compressed_sha256": digest(raw), "compressed_size": len(raw),
                      "decoded_sha256": digest(decoded), "decoded_size": len(decoded)})
    return {"format": 1, "captured_at": datetime.now(timezone.utc).isoformat(),
            "response_encoding": "base64", "responses": responses, "cases": cases,
            "legal_approval": False, "release_ready": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.resolve().is_relative_to(Path(__file__).resolve().parents[2]):
        raise ValueError("retain original metadata outside the repository")
    if args.output.exists():
        raise ValueError("output already exists")
    data = (json.dumps(capture(), indent=2) + "\n").encode("utf-8")
    with args.output.open("xb") as output:
        output.write(data)
    print(json.dumps({"fixture_sha256": digest(data), "size": len(data), "cases": 3,
                      "legal_approval": False, "release_ready": False}))


if __name__ == "__main__":
    main()
