import json
import unittest
import capture_java_metadata as capture


class JavaCapture(unittest.TestCase):
    def data(self, url):
        route = url.removeprefix("https://mise-java.jdx.dev/jvm/ga/").removesuffix(".json")
        suffix, kind = {"linux/x86_64": ("x64_linux", "tar.gz"),
                        "macosx/aarch64": ("aarch64_mac", "tar.gz"),
                        "windows/x86_64": ("x64_windows", "zip")}[route]
        return {"vendor": "temurin", "version": "21.0.6+7.0.LTS", "image_type": "jdk",
                "jvm_impl": "hotspot", "features": [], "file_type": kind,
                "java_version": "21.0.6+7-LTS", "checksum": "sha256:" + "a" * 64,
                "url": "https://github.com/adoptium/temurin21-binaries/releases/download/"
                       "jdk-21.0.6%2B7/OpenJDK21U-jdk_" + suffix + "_hotspot_21.0.6_7." + kind}

    def test_targets_and_original_bytes(self):
        result = capture.capture(lambda url: json.dumps([self.data(url)]).encode())
        self.assertEqual(len(result["cases"]), 3)
        self.assertFalse(result["legal_approval"])
        for case in result["cases"]:
            self.assertEqual(case["catalog_size"], len(result["responses"][case["catalog_url"]].encode()))

    def test_wrong_variant_target_checksum_or_identity_is_rejected(self):
        for field, value in [("version", "21.0.6+7"), ("image_type", "jre"),
                             ("features", ["musl"]), ("file_type", "msi"),
                             ("checksum", "sha256:invalid"), ("url", "https://example.invalid/a")]:
            def acquire(url):
                row = self.data(url)
                row[field] = value
                return json.dumps([row]).encode()
            with self.subTest(field=field), self.assertRaises(ValueError):
                capture.capture(acquire)

    def test_duplicate_missing_invalid_encoding_and_oversize_are_rejected(self):
        for acquire in [lambda url: json.dumps([self.data(url)] * 2).encode(),
                        lambda _: b"[]", lambda _: b"{}", lambda _: b"\xff",
                        lambda _: b" " * (capture.LIMIT + 1)]:
            with self.assertRaises(ValueError):
                capture.capture(acquire)


if __name__ == "__main__":
    unittest.main()
