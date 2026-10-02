import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("upstream_check", Path(__file__).with_name("check.py"))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class MaintenanceTests(unittest.TestCase):
    def fixture(self):
        rev = "a" * 40
        return ({"dependencies": {"mise": {"git": check.FORK, "rev": rev, "default-features": False}}},
                {"package": [{"name": "mise", "source": f"git+{check.FORK}?rev={rev}#{rev}"}]})

    def test_exact_public_pin_and_lock_must_agree(self):
        manifest, lock = self.fixture()
        self.assertEqual(check.pinned_dependency(manifest, lock), "a" * 40)
        lock["package"][0]["source"] += "changed"
        with self.assertRaises(ValueError):
            check.pinned_dependency(manifest, lock)

    def test_moving_or_wrong_sources_fail(self):
        for field, value in [("rev", "main"), ("branch", "main"), ("git", "https://github.com/jdx/mise"), ("default-features", True)]:
            manifest, lock = self.fixture()
            manifest["dependencies"]["mise"][field] = value
            with self.assertRaises(ValueError):
                check.pinned_dependency(manifest, lock)

    def test_unintegrated_fork_is_not_a_production_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[dependencies]\n")
            (root / "Cargo.lock").write_text("version = 4\n")
            def api(path):
                if path.endswith("releases/latest"):
                    return {"tag_name": "v1", "draft": False, "prerelease": False, "published_at": "2026-10-02T00:00:00Z"}
                if path == "repos/oyzuai/mise":
                    return {"private": False, "parent": {"full_name": "jdx/mise"}, "default_branch": "main"}
                return {"sha": "b" * 40}
            report = check.observe(root, api)
            self.assertIsNone(report["production_pin"])
            self.assertFalse(report["release_ready"])
            self.assertEqual(report["status"], "production-integration-not-configured")


if __name__ == "__main__":
    unittest.main()
