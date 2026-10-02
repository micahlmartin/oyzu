import json
from pathlib import Path
import tempfile
import unittest

from capture_go_metadata import CATALOG, TARGETS, capture, digest, write_fixture


class CaptureTests(unittest.TestCase):
    def test_output_identity_exclusivity_and_replay_limit(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "fixture.json"
            expected_digest = write_fixture(output, {"fixture": "original"})
            original = output.read_bytes()
            self.assertEqual(expected_digest, digest(original))
            with self.assertRaises(FileExistsError):
                write_fixture(output, {"fixture": "replacement"})
            self.assertEqual(output.read_bytes(), original)
            oversized = Path(temporary) / "oversized.json"
            # JSON escaping can exceed the replay limit even when the captured
            # UTF-8 body itself was within its 16 MiB input limit.
            with self.assertRaisesRegex(ValueError, "replay byte limit"):
                write_fixture(oversized, {"body": "\x00" * (6 * 1024 * 1024)})
            self.assertFalse(oversized.exists())

    def records(self):
        return [{"version": "go1.24.13", "stable": True, "files": [
            {"filename": f"go1.24.13.{os_name}-{arch}.{extension}", "os": os_name,
             "arch": arch, "version": "go1.24.13", "kind": "archive", "size": 42,
             "sha256": "a" * 64} for _, os_name, arch, extension in TARGETS]}]

    def test_exact_capture_and_bytes(self):
        raw = (json.dumps(self.records()) + "\r\n").encode()
        calls = []

        def acquire(url, limit):
            calls.append((url, limit))
            return raw if url == CATALOG else b"A" * 64 + b"\r\n"

        result = capture(["1.24.13"], acquire)
        self.assertEqual(len(calls), 4)
        self.assertEqual(len(result["expected"]), 3)
        self.assertEqual(result["responses"][CATALOG]["body_utf8"].encode(), raw)
        self.assertEqual(result["expected"][0]["catalog_sha256"], digest(raw))
        self.assertFalse(result["legal_approval"])

    def test_bad_requests_never_acquire(self):
        for versions in ([], ["1.24"], ["01.24.13"], ["1.24.13"] * 2):
            with self.subTest(versions=versions), self.assertRaises(ValueError):
                capture(versions, lambda *_: self.fail("unexpected acquisition"))

    def test_record_and_sidecar_failures(self):
        for mutation in (
            lambda r: r.clear(),
            lambda r: r.append(r[0]),
            lambda r: r[0].update(stable=False),
            lambda r: r[0]["files"].pop(),
            lambda r: r[0]["files"][0].update(size=True),
            lambda r: r[0]["files"][0].update(arch="wrong"),
            lambda r: r[0]["files"][0].update(sha256="invalid"),
        ):
            records = self.records()
            mutation(records)
            with self.assertRaises(ValueError):
                capture(["1.24.13"], lambda url, _: json.dumps(records).encode()
                        if url == CATALOG else b"a" * 64)
        with self.assertRaisesRegex(ValueError, "disagreement"):
            capture(["1.24.13"], lambda url, _: json.dumps(self.records()).encode()
                    if url == CATALOG else b"b" * 64)
        with self.assertRaisesRegex(ValueError, "byte limit"):
            capture(["1.24.13"], lambda *_: b" " * (16 * 1024 * 1024 + 1))
        with self.assertRaises(UnicodeDecodeError):
            capture(["1.24.13"], lambda *_: b"\xff")


if __name__ == "__main__":
    unittest.main()
