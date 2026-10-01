"""Adapter boundary regressions; full native builds run in test-build-scenarios.py."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import warnings
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('oyzu_python', ROOT / 'src/helpers/python.py')
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


class WheelMetadataTests(unittest.TestCase):
    def read(self, entries):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'demo_pkg-1.0-py3-none-any.whl'
            with zipfile.ZipFile(path, 'w') as wheel, warnings.catch_warnings():
                warnings.simplefilter('ignore', UserWarning)
                for name, body in entries:
                    wheel.writestr(name, body)
            return adapter.wheel_metadata(path)

    def test_vendored_metadata_is_not_the_distribution_identity(self):
        info = self.read([
            ('demo_pkg/_vendor/other-9.dist-info/METADATA', 'Name: other\nVersion: 9\n'),
            ('demo_pkg-1.0.dist-info/METADATA', 'Name: demo-pkg\nVersion: 1.0\n'),
        ])
        self.assertEqual(info['Name'], 'demo-pkg')

    def test_missing_duplicate_and_mismatched_metadata_are_rejected(self):
        valid = ('demo_pkg-1.0.dist-info/METADATA', 'Name: demo-pkg\nVersion: 1.0\n')
        cases = [[], [valid, valid],
                 [('other-1.0.dist-info/METADATA', valid[1])],
                 [(valid[0], 'Name: other\nVersion: 1.0\n')],
                 [(valid[0], 'Name: demo-pkg\nVersion: 2.0\n')],
                 [(valid[0], 'Name: demo-pkg\nName: other\nVersion: 1.0\n')],
                 [('nested/' + valid[0], valid[1])]]
        for entries in cases:
            with self.subTest(entries=entries), self.assertRaises(ValueError):
                self.read(entries)


if __name__ == '__main__':
    unittest.main()
