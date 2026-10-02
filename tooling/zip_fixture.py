"""Independent ZIP inventory oracle for pinned native archive qualification.

Hashes every original file without extraction or execution. Includes implicit
parent directories so the inventory describes the materialized payload tree.
"""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

def prepare(archive: Path, output: Path, name: str, digest: str, size: int, prefix: str, version: str):
    if archive.resolve() == output.resolve():
        raise ValueError("manifest must not replace the archive")
    if archive.stat().st_size != size:
        raise ValueError("unexpected archive size")
    with archive.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != digest:
            raise ValueError("unexpected archive digest")
    entries = {}
    observed = set()
    with zipfile.ZipFile(archive) as source:
        for item in source.infolist():
            if not item.filename.startswith(prefix):
                raise ValueError("unexpected archive root")
            path = item.filename[len(prefix):].rstrip("/")
            if not path:
                continue
            if path in observed:
                raise ValueError("duplicate archive entry")
            observed.add(path)
            parts = path.split("/")
            for count in range(1, len(parts)):
                parent = "/".join(parts[:count])
                entries.setdefault(parent, {"type": "directory", "size": None, "digest": None})
            if item.is_dir():
                entries[path] = {"type": "directory", "size": None, "digest": None}
            else:
                with source.open(item) as stream:
                    file_digest = hashlib.file_digest(stream, "sha256").hexdigest()
                entries[path] = {"type": "file", "size": item.file_size, "digest": "sha256:" + file_digest}
    result = {"archive": name, "archive_digest": "sha256:" + digest,
              "archive_size": size, "version": version, "entries": entries}
    with output.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(result, stream, sort_keys=True, indent=2)
        stream.write("\n")
    print(f"Inventoried {len(entries)} real archive entries; no extraction or execution")


def run_cli(name, digest, size, prefix, version):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    args = parser.parse_args()
    prepare(args.archive, args.manifest, name, digest, size, prefix, version)
