#!/usr/bin/env python3
"""Verify pinned jq raw fixture and inventory bytes without executing it.

Original artifact and upstream notices are retained outside the checkout.
Declared checksum consistency is not signature verification or source admission.
"""
import argparse
import hashlib
import json
from pathlib import Path

DIGEST = "23cb60a1354eed6bcc8d9b9735e8c7b388cd1fdcb75726b93bc299ef22dd9334"
SIZE = 1026560


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    args = parser.parse_args()
    if args.archive.stat().st_size != SIZE:
        raise ValueError("unexpected jq artifact size")
    with args.archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if args.archive.stat().st_size != SIZE or digest != DIGEST:
        raise ValueError("unexpected jq artifact identity")
    result = {"archive": "jq-windows-amd64.exe", "archive_digest": "sha256:" + DIGEST,
              "archive_size": SIZE, "version": "1.8.1",
              "entries": {"jq.exe": {"type": "file", "size": SIZE, "digest": "sha256:" + DIGEST}}}
    with args.manifest.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(result, output, sort_keys=True, indent=2)
        output.write("\n")
    print("Inventoried pinned raw jq artifact; no extraction or execution")


if __name__ == "__main__":
    main()
