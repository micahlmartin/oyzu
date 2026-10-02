"""Offline consistency checks for supplemental notices; never legal approval."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys

from verify_notices import bounded_read, require, sha, unapproved

ROOT = Path(__file__).resolve().parents[2]


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate evidence field")
        result[key] = value
    return result


def document(path):
    with path.open("rb") as stream:
        data = bounded_read(stream, 1024 * 1024)
    return data, json.loads(data, object_pairs_hook=unique)


def verify(artifacts, index_path, pointers_path):
    _, index = document(index_path)
    pointer_bytes, pointers = document(pointers_path)
    unapproved(index)
    unapproved(pointers)
    require(index["format"] == pointers["format"] == 1, "unsupported evidence format")
    require(sha(pointer_bytes) == index["source_pointer_sha256"], "source pointer hash mismatch")
    packages = {p["id"]: p for p in pointers["packages"]}
    require(len(packages) == len(pointers["packages"]), "duplicate source package")
    seen_sources, seen_paths = set(), set()
    count = total = 0
    require(len(index["sources"]) <= 4096, "source count exceeded")
    for source in index["sources"]:
        repository, revision = source["repository"], source["revision"]
        match = re.fullmatch(r"https://github\.com/([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)", repository)
        require(match is not None and re.fullmatch(r"[0-9a-f]{40}", revision), "invalid source identity")
        slug = match.group(1)
        require(all(part not in (".", "..") for part in slug.split("/")), "unsafe repository path")
        identity = (repository, revision)
        require(identity not in seen_sources, "duplicate source identity")
        seen_sources.add(identity)
        require(source["package_ids"] and len(set(source["package_ids"])) == len(source["package_ids"]), "invalid package references")
        for package_id in source["package_ids"]:
            package = packages[package_id]
            require(package["repository"] == repository and package["vcs"]["git"]["sha1"] == revision,
                    "package/source binding mismatch")
        for notice in source["files"]:
            path = notice["path"]
            parts = PurePosixPath(path).parts
            require(parts and str(PurePosixPath(path)) == path and not path.startswith("/")
                    and all(part not in (".", "..") and ":" not in part and "\\" not in part for part in parts),
                    "unsafe notice path")
            relative = f"{slug}/{revision}/{path}"
            require(notice["retained_path"] == relative and relative not in seen_paths, "invalid retained path")
            seen_paths.add(relative)
            require(notice["url"] == f"https://raw.githubusercontent.com/{slug}/{revision}/{path}", "notice URL mismatch")
            candidate = artifacts
            for part in PurePosixPath(relative).parts:
                candidate = candidate / part
                require(not candidate.is_symlink(), "redirected notice path")
            require(candidate.resolve().is_relative_to(artifacts.resolve()), "notice escaped retention root")
            require(candidate.is_file(), "notice is not a regular file")
            with candidate.open("rb") as stream:
                data = bounded_read(stream, 2 * 1024 * 1024)
            count += 1
            total += len(data)
            require(count <= 20000 and total <= 256 * 1024 * 1024, "notice budget exceeded")
            require(type(notice["size"]) is int and len(data) == notice["size"]
                    and sha(data) == notice["sha256"], "notice bytes mismatch")
            git_hash = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
            require(git_hash == notice["git_blob_sha1"], "Git blob mismatch")
    return {"sources": len(seen_sources), "files": count, "bytes": total,
            "verified": True, "legal_approval": False, "release_ready": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--index", type=Path, default=ROOT / "docs/proposals/OEP-0003-mise-integration/supplemental-notice-evidence.json")
    parser.add_argument("--pointers", type=Path, default=ROOT / "docs/proposals/OEP-0003-mise-integration/missing-notice-source-pointers.json")
    args = parser.parse_args()
    print(json.dumps(verify(args.artifacts, args.index, args.pointers), indent=2))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError) as error:
        print("Supplemental notice verification failed: " + type(error).__name__, file=sys.stderr)
        sys.exit(1)
