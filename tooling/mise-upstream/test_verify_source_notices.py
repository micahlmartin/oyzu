import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from verify_source_notices import github_repository, verify


class SourceNoticeTests(unittest.TestCase):
    def test_repository_spellings_preserve_repository_and_pinned_revision(self):
        canonical = "https://github.com/example/project"
        for suffix in ("", "/", ".git", ".git/", "/tree/main/crates/tool", "/tree/master/tool/"):
            self.assertEqual(github_repository(canonical + suffix), canonical)
        for value in (canonical + "/tree/main", canonical + "/tree/main/../other",
                      canonical + "/tree/main//tool", canonical + "?ref=other",
                      "http://github.com/example/project", "https://github.com.evil/example/project",
                      "https://user@github.com/example/project", "https://github.com/../project",
                      canonical + "/issues/1", canonical + "/tree/main/%2e%2e"):
            with self.assertRaises(ValueError):
                github_repository(value)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            index, pointer_path, _ = self.fixture(root)
            pointers = json.loads(pointer_path.read_text())
            pointers["packages"][0]["repository"] += "/tree/main/crates/tool"
            pointer_path.write_text(json.dumps(pointers))
            index["source_pointer_sha256"] = hashlib.sha256(pointer_path.read_bytes()).hexdigest()
            index_path = root / "index.json"
            index_path.write_text(json.dumps(index))
            self.assertTrue(verify(root, index_path, pointer_path)["verified"])
            pointers["packages"][0]["vcs"]["git"]["sha1"] = "b" * 40
            pointer_path.write_text(json.dumps(pointers))
            index["source_pointer_sha256"] = hashlib.sha256(pointer_path.read_bytes()).hexdigest()
            index_path.write_text(json.dumps(index))
            with self.assertRaisesRegex(ValueError, "binding mismatch"):
                verify(root, index_path, pointer_path)

    def fixture(self, root):
        revision = "a" * 40
        repository = "https://github.com/example/project"
        relative = f"example/project/{revision}/LICENSE"
        data = b"Synthetic test notice\r\n"
        artifact = root / relative
        artifact.parent.mkdir(parents=True)
        artifact.write_bytes(data)
        pointers = {"format": 1, "legal_approval": False, "release_ready": False,
                    "packages": [{"id": "example@1", "repository": repository,
                                  "vcs": {"git": {"sha1": revision}}}]}
        pointer_path = root / "pointers.json"
        pointer_path.write_text(json.dumps(pointers))
        index = {"format": 1, "legal_approval": False, "release_ready": False,
                 "source_pointer_sha256": hashlib.sha256(pointer_path.read_bytes()).hexdigest(),
                 "sources": [{"repository": repository, "revision": revision, "package_ids": ["example@1"],
                              "files": [{"path": "LICENSE", "retained_path": relative,
                                         "url": f"https://raw.githubusercontent.com/example/project/{revision}/LICENSE",
                                         "size": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                                         "git_blob_sha1": hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()}]}]}
        return index, pointer_path, artifact

    def test_original_bytes_and_missing_or_changed_notices(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            index, pointers, artifact = self.fixture(root)
            index_path = root / "index.json"
            index_path.write_text(json.dumps(index))
            result = verify(root, index_path, pointers)
            self.assertEqual(result["files"], 1)
            self.assertEqual(result["packages_with_candidates"], 1)
            self.assertEqual(result["packages_without_candidates"], 0)
            self.assertFalse(result["legal_approval"])
            artifact.write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "bytes mismatch"):
                verify(root, index_path, pointers)
            artifact.unlink()
            with self.assertRaises(ValueError):
                verify(root, index_path, pointers)

    def test_approval_bindings_paths_and_duplicates_fail(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            original, pointers, _ = self.fixture(root)
            index_path = root / "index.json"
            for mutate in (
                lambda d: d.update(legal_approval=True),
                lambda d: d.update(source_pointer_sha256="0" * 64),
                lambda d: d["sources"][0].update(revision="b" * 40),
                lambda d: d["sources"][0].update(files=[]),
                lambda d: d["sources"][0]["files"][0].update(path="../LICENSE"),
                lambda d: d["sources"][0]["files"][0].update(retained_path="../LICENSE"),
                lambda d: d["sources"][0]["files"][0].update(git_blob_sha1="0" * 40),
                lambda d: d["sources"].append(copy.deepcopy(d["sources"][0])),
                lambda d: d["sources"][0]["files"].append(copy.deepcopy(d["sources"][0]["files"][0])),
            ):
                index = copy.deepcopy(original)
                mutate(index)
                index_path.write_text(json.dumps(index))
                with self.assertRaises(ValueError):
                    verify(root, index_path, pointers)
            index_path.write_text('{"format":1,"format":1}')
            with self.assertRaisesRegex(ValueError, "duplicate"):
                verify(root, index_path, pointers)


if __name__ == "__main__":
    unittest.main()
