"""Check replay byte preservation and independent gzip capture limits."""
import base64
import gzip
import unittest

import capture_python_metadata as capture


class PythonCatalogCaptureTests(unittest.TestCase):
    def test_preserves_original_compressed_bytes_and_both_digests(self):
        raw = gzip.compress(b"cpython-example\n", mtime=123)
        fixture = capture.capture(lambda _: raw)
        self.assertEqual(len(fixture["cases"]), 3)
        self.assertEqual(fixture["response_encoding"], "base64")
        for case in fixture["cases"]:
            saved = base64.b64decode(fixture["responses"][case["catalog_url"]], validate=True)
            self.assertEqual(saved, raw)
            self.assertEqual(case["compressed_sha256"], capture.digest(raw))
            self.assertEqual(case["decoded_sha256"], capture.digest(gzip.decompress(raw)))
        self.assertFalse(fixture["legal_approval"])
        self.assertFalse(fixture["release_ready"])

    def test_compressed_and_decoded_overflow(self):
        with self.assertRaisesRegex(ValueError, "compressed"):
            capture.decode(b"x" * 101, 100)
        raw = gzip.compress(b"x" * 100)
        self.assertEqual(capture.decode(raw, 100), b"x" * 100)
        with self.assertRaisesRegex(ValueError, "decoded"):
            capture.decode(raw, 99)

    def test_rejects_corruption_truncation_and_invalid_utf8(self):
        raw = gzip.compress(b"catalog\n")
        corrupt = bytearray(raw)
        corrupt[-8] ^= 1
        for candidate in (raw[:-1], bytes(corrupt), gzip.compress(b"\xff")):
            with self.subTest(candidate=candidate), self.assertRaises((OSError, EOFError, UnicodeError)):
                capture.decode(candidate)

    def test_empty_catalog_and_acquisition_failure_produce_no_fixture(self):
        with self.assertRaisesRegex(ValueError, "empty"):
            capture.capture(lambda _: gzip.compress(b" \n"))
        def fail(_):
            raise OSError("offline")
        with self.assertRaisesRegex(OSError, "offline"):
            capture.capture(fail)


if __name__ == "__main__":
    unittest.main()
