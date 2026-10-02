#!/usr/bin/env python3
"""Independently inventory the pinned real Node ZIP for store qualification.

No downloads, installation or execution. Keep archive and output outside the repo.
The checksum is from the release's HTTPS SHASUMS256.txt, not a signature claim.
"""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

NAME = "node-v22.14.0-win-x64.zip"
DIGEST = "55b639295920b219bb2acbcfa00f90393a2789095b7323f79475c9f34795f217"
SIZE = 34906389
PREFIX = "node-v22.14.0-win-x64/"


def prepare(archive: Path, output: Path):
    if archive.resolve() == output.resolve():
        raise ValueError("manifest must not replace the archive")
    if archive.stat().st_size != SIZE:
        raise ValueError("unexpected archive size")
    with archive.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != DIGEST:
            raise ValueError("unexpected archive digest")
    entries = {}
    with zipfile.ZipFile(archive) as source:
        for item in source.infolist():
            if not item.filename.startswith(PREFIX):
                raise ValueError("unexpected archive root")
            path = item.filename[len(PREFIX):].rstrip("/")
            if not path:
                continue
            if path in entries:
                raise ValueError("duplicate archive entry")
            if item.is_dir():
                entries[path] = {"type": "directory", "size": None, "digest": None}
            else:
                with source.open(item) as stream:
                    digest = hashlib.file_digest(stream, "sha256").hexdigest()
                entries[path] = {"type": "file", "size": item.file_size, "digest": "sha256:" + digest}
    result = {"archive": NAME, "archive_digest": "sha256:" + DIGEST,
              "archive_size": SIZE, "version": "22.14.0", "entries": entries}
    with output.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(result, stream, sort_keys=True, indent=2)
        stream.write("\n")
    print(f"Inventoried {len(entries)} real archive entries; no extraction or execution")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    args = parser.parse_args()
    prepare(args.archive, args.manifest)
