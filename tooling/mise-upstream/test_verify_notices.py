"""Independent retained-evidence verification failure paths."""

import json
from pathlib import Path
import tempfile
import unittest
import zipfile

from verify_notices import sha, verify_target


class NoticeVerificationTests(unittest.TestCase):
    def fixture(self, directory, mutation=None):
        data = b"Synthetic notice fixture\r\n"
        normalized = sha(data.replace(b"\r\n", b"\n"))
        report = {"legal_approval": False, "release_ready": False,
                  "tracked_worktree_changes": False, "target": "test-target",
                  "source_revision": "test-revision", "package_count": 1,
                  "packages": [{"id": "test-package", "notice_files": [
                      {"path": "LICENSE", "sha256_lf": normalized}]}]}
        report_bytes = json.dumps(report).encode()
        index = {"format": 1, "legal_approval": False, "release_ready": False,
                 "report_sha256": sha(report_bytes), "notice_count": 1,
                 "notice_bytes": len(data), "packages_without_observed_notices": [],
                 "notices": [{"package_id": "test-package", "package_path": "LICENSE",
                              "archive_path": "notices/test/LICENSE", "size": len(data),
                              "sha256": sha(data), "sha256_lf": normalized}]}
        entries = {"cargo-evidence.json": report_bytes, "notices/test/LICENSE": data}
        if mutation:
            mutation(index, entries)
        entries["INDEX.json"] = json.dumps(index).encode()
        (directory / "cargo-evidence.json").write_bytes(report_bytes)
        with zipfile.ZipFile(directory / "cargo-notices.zip", "w") as archive:
            for name, content in entries.items():
                archive.writestr(name, content)
        return {"report_sha256": sha(report_bytes),
                "archive_sha256": sha((directory / "cargo-notices.zip").read_bytes()),
                "target": "test-target", "source_revision": "test-revision",
                "packages": 1, "missing": 0, "notices": 1, "notice_bytes": len(data)}

    def test_valid_and_external_tampering(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            expected = self.fixture(directory)
            self.assertTrue(verify_target(directory, expected)["verified"])
            self.assertFalse(verify_target(directory, expected)["legal_approval"])
            expected["archive_sha256"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "archive hash mismatch"):
                verify_target(directory, expected)
            expected = self.fixture(directory)
            (directory / "cargo-evidence.json").write_bytes(b"{}")
            with self.assertRaisesRegex(ValueError, "report hash mismatch"):
                verify_target(directory, expected)

    def test_internal_inconsistency_even_with_updated_archive_hash(self):
        mutations = [
            (lambda i, e: i.update(legal_approval=True), "unapproved"),
            (lambda i, e: e.update({"cargo-evidence.json": b"{}"}), "embedded report"),
            (lambda i, e: e.update({"extra": b"hidden"}), "unindexed"),
            (lambda i, e: e.update({"notices/test/LICENSE": b"tampered"}), "bytes/hash"),
            (lambda i, e: i["notices"][0].update(package_id="wrong"), "identities"),
            (lambda i, e: i["notices"].append(dict(i["notices"][0])), "duplicate indexed"),
            (lambda i, e: i.update(notice_count=2), "count mismatch"),
            (lambda i, e: i.update(packages_without_observed_notices=["wrong"]), "identities"),
        ]
        for mutation, diagnostic in mutations:
            with self.subTest(diagnostic=diagnostic), tempfile.TemporaryDirectory() as temporary:
                directory = Path(temporary)
                expected = self.fixture(directory, mutation)
                with self.assertRaisesRegex(ValueError, diagnostic):
                    verify_target(directory, expected)


if __name__ == "__main__":
    unittest.main()
