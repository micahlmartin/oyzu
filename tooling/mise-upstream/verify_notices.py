"""Recheck retained candidate evidence offline; never grants legal approval.

The checked-in evidence index is the trusted hash anchor. Inputs are read without
extracting archives or executing contents. This verifies evidence consistency,
not license meaning, authenticity of the anchor, or completeness of a release.
"""

import argparse
import hashlib
import json
from pathlib import Path
import sys
import zipfile


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def bounded_read(stream, limit):
    data = stream.read(limit + 1)
    require(len(data) <= limit, "evidence exceeds byte limit")
    return data


def unapproved(document):
    require(document.get("legal_approval") is False
            and document.get("release_ready") is False,
            "candidate evidence must remain explicitly unapproved")


def verify_target(directory, expected):
    """Verify one retained artifact against a trusted target record, read-only."""
    with (directory / "cargo-evidence.json").open("rb") as stream:
        report_bytes = bounded_read(stream, 64 * 1024 * 1024)
    require(sha(report_bytes) == expected["report_sha256"], "report hash mismatch")
    report = json.loads(report_bytes)
    unapproved(report)
    require(report.get("tracked_worktree_changes") is False, "dirty source evidence")
    for field in ("target", "source_revision"):
        require(report[field] == expected[field], f"{field} mismatch")
    packages = report["packages"]
    require(len(packages) == report["package_count"] == expected["packages"],
            "package count mismatch")
    package_ids = [package["id"] for package in packages]
    require(len(set(package_ids)) == len(package_ids), "duplicate package")
    report_notices = {}
    for package in packages:
        for notice in package["notice_files"]:
            key = (package["id"], notice["path"])
            require(key not in report_notices, "duplicate report notice")
            report_notices[key] = notice["sha256_lf"]
    missing = sorted(package["id"] for package in packages if not package["notice_files"])
    require(len(missing) == expected["missing"], "missing-notice count mismatch")
    with (directory / "cargo-notices.zip").open("rb") as stream:
        digest = hashlib.sha256()
        total = 0
        while chunk := stream.read(1024 * 1024):
            total += len(chunk)
            require(total <= 400 * 1024 * 1024, "archive exceeds byte limit")
            digest.update(chunk)
        require(digest.hexdigest() == expected["archive_sha256"], "archive hash mismatch")
        stream.seek(0)
        with zipfile.ZipFile(stream) as archive:
            entries = archive.infolist()
            names = [entry.filename for entry in entries]
            require(len(entries) <= 20002 and len(set(names)) == len(names),
                    "duplicate or excessive archive entries")
            require(all(entry.compress_type == zipfile.ZIP_STORED
                        and not entry.flag_bits & 1 for entry in entries),
                    "unexpected compression or encryption")

            def read(name, limit):
                require(archive.getinfo(name).file_size <= limit, "entry exceeds byte limit")
                with archive.open(name) as source:
                    return bounded_read(source, limit)

            require(read("cargo-evidence.json", 64 * 1024 * 1024) == report_bytes,
                    "embedded report differs")
            index = json.loads(read("INDEX.json", 64 * 1024 * 1024))
            require(index["format"] == 1, "unsupported archive index")
            unapproved(index)
            require(index["report_sha256"] == expected["report_sha256"], "index report hash mismatch")
            require(index["packages_without_observed_notices"] == missing,
                    "missing-notice identities mismatch")
            observed = {}
            expected_names = {"INDEX.json", "cargo-evidence.json"}
            notice_bytes = 0
            for notice in index["notices"]:
                key = (notice["package_id"], notice["package_path"])
                require(key not in observed, "duplicate indexed notice")
                path = notice["archive_path"]
                require(path not in expected_names, "duplicate indexed archive path")
                expected_names.add(path)
                data = read(path, 2 * 1024 * 1024)
                notice_bytes += len(data)
                require(notice_bytes <= 256 * 1024 * 1024, "notice total exceeds byte limit")
                require(len(data) == notice["size"] and sha(data) == notice["sha256"]
                        and sha(data.replace(b"\r\n", b"\n")) == notice["sha256_lf"],
                        "notice bytes/hash mismatch")
                observed[key] = notice["sha256_lf"]
            require(observed == report_notices, "report/index notice identities mismatch")
            require(set(names) == expected_names, "unindexed archive entry")
            require(len(observed) == index["notice_count"] == expected["notices"],
                    "notice count mismatch")
            require(notice_bytes == index["notice_bytes"] == expected["notice_bytes"],
                    "notice byte total mismatch")
    return {"target": expected["target"], "verified": True,
            "legal_approval": False, "release_ready": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True,
                        help="directory containing downloaded named artifact directories")
    parser.add_argument("--index", type=Path, default=Path(__file__).resolve().parents[2]
                        / "docs/proposals/OEP-0003-mise-integration/candidate-notice-evidence.json")
    args = parser.parse_args()
    with args.index.open("rb") as stream:
        index = json.loads(bounded_read(stream, 1024 * 1024))
    require(index["format"] == 1, "unsupported evidence index")
    unapproved(index)
    results = []
    for target in index["targets"]:
        name = target["artifact"]
        require(name and name not in (".", "..")
                and not any(char in name for char in "/\\:"), "invalid artifact directory")
        results.append(verify_target(args.artifacts / name, target))
    require(bool(results), "empty target evidence")
    print(json.dumps({"targets": results, "legal_approval": False, "release_ready": False}, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, zipfile.BadZipFile) as error:
        print(f"Evidence verification failed: {error}", file=sys.stderr)
        sys.exit(1)
