"""Retain bounded Python precompiled catalogs for replay; no install or approval."""
import argparse
import base64
from datetime import datetime, timezone
import gzip
import hashlib
import io
import json
import re
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


def capture_checksums(fixture, acquire):
    """Independently select a fixed CPython release family for metadata replay."""
    manifests = {}
    for case in fixture["cases"]:
        url = case["catalog_url"]
        platform = url.rsplit("python-precompiled-", 1)[1][:-3]
        body = decode(base64.b64decode(fixture["responses"][url], validate=True)).decode()
        pattern = re.compile(r"cpython-3\.12\.13\+(\d{8})-" + re.escape(platform)
                             + r"-install_only_stripped\.tar\.gz")
        candidates = [(match[1], line) for line in body.splitlines()
                      if (match := pattern.fullmatch(line))]
        if not candidates:
            raise ValueError("qualified Python replay artifact missing")
        release, filename = max(candidates)
        checksum_url = ("https://github.com/astral-sh/python-build-standalone/releases/download/"
                        + release + "/SHA256SUMS")
        if checksum_url not in manifests:
            raw = acquire(checksum_url)
            if len(raw) > 8 * 1024 * 1024:
                raise ValueError("checksum manifest exceeds byte limit")
            records = {}
            for line in raw.decode("utf-8", errors="strict").splitlines():
                if not line.strip():
                    continue
                fields = line.split()
                if len(fields) != 2 or not re.fullmatch(r"[0-9a-fA-F]{64}", fields[0]):
                    raise ValueError("invalid checksum entry")
                name = fields[1].removeprefix("*")
                if not name or name in records or len(records) >= 4096:
                    raise ValueError("duplicate or excessive checksum entries")
                records[name] = fields[0].lower()
            manifests[checksum_url] = (raw, records)
            fixture["responses"][checksum_url] = base64.b64encode(raw).decode("ascii")
        raw, records = manifests[checksum_url]
        if filename not in records:
            raise ValueError("selected Python artifact missing from checksums")
        case.update({"version": "3.12.13", "filename": filename, "release": release,
                     "checksum_url": checksum_url, "checksum_size": len(raw),
                     "checksum_sha256": digest(raw),
                     "declared_sha256": "sha256:" + records[filename]})


def capture(acquire=fetch, with_checksums=False):
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
    fixture = {"format": 1, "captured_at": datetime.now(timezone.utc).isoformat(),
               "response_encoding": "base64", "responses": responses, "cases": cases,
               "legal_approval": False, "release_ready": False}
    if with_checksums:
        capture_checksums(fixture, acquire)
    return fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--with-checksums", action="store_true")
    args = parser.parse_args()
    if args.output.resolve().is_relative_to(Path(__file__).resolve().parents[2]):
        raise ValueError("retain original metadata outside the repository")
    if args.output.exists():
        raise ValueError("output already exists")
    data = (json.dumps(capture(with_checksums=args.with_checksums), indent=2) + "\n").encode("utf-8")
    with args.output.open("xb") as output:
        output.write(data)
    print(json.dumps({"fixture_sha256": digest(data), "size": len(data), "cases": 3,
                      "legal_approval": False, "release_ready": False}))


if __name__ == "__main__":
    main()
