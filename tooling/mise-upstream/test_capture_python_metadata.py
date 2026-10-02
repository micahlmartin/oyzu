"""Check replay byte preservation and independent gzip capture limits."""
import base64
import gzip
import unittest

import capture_python_metadata as capture


class PythonCatalogCaptureTests(unittest.TestCase):
    def test_checksum_capture_and_failures(self):
        names = [f"cpython-3.12.13+20260807-{triple}-install_only_stripped.tar.gz"
                 for _, triple in capture.TARGETS]
        checksums = "".join("a" * 64 + "  " + name + "\n" for name in names).encode()
        calls = []
        def acquire(url):
            calls.append(url)
            if url.endswith("SHA256SUMS"):
                return checksums
            return gzip.compress(("\n".join(names) + "\n").encode())
        fixture = capture.capture(acquire, with_checksums=True)
        self.assertEqual(len(calls), 4)
        for case, name in zip(fixture["cases"], names):
            self.assertEqual(case["filename"], name)
            self.assertEqual(case["declared_sha256"], "sha256:" + "a" * 64)
            self.assertEqual(base64.b64decode(fixture["responses"][case["checksum_url"]]), checksums)
        for malformed in (checksums * 2, b"", b"bad  file\n", b"\xff", b"x" * (8 * 1024 * 1024 + 1)):
            checksums = malformed
            with self.subTest(size=len(malformed)), self.assertRaises((ValueError, UnicodeError)):
                capture.capture(acquire, with_checksums=True)

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
