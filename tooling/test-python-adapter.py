"""Adapter boundary regressions; full native builds run in test-build-scenarios.py."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import warnings
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('oyzu_python', ROOT / 'src/builders/python/runtime/adapter.py')
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


class WheelMetadataTests(unittest.TestCase):
    def test_quality_defaults_preserve_declared_and_locked_versions(self):
        previous = Path.cwd()
        with tempfile.TemporaryDirectory() as temporary, patch.dict(os.environ, {'OYZU_PYTHON_LINTER':'flake8', 'OYZU_PYTHON_FORMATTER':'black'}):
            root = Path(temporary)
            (root/'requirements.txt').write_text('black==24.10.0\n')
            try:
                os.chdir(root)
                requirements, _ = adapter.requirement_lines(['flake8==7.2.0'])
                self.assertEqual(requirements, ['black==24.10.0', 'pytest==8.3.5', 'pytest-cov==6.0.0'])
                requirements, _ = adapter.requirement_lines()
                self.assertIn('flake8==7.3.0', requirements)
                self.assertNotIn('black==25.1.0', requirements)
                os.environ['OYZU_PYTHON_LINTER'] = 'unrecognized'
                with self.assertRaises(ValueError):
                    adapter.requirement_lines()
            finally:
                os.chdir(previous)

    def test_runtime_role_survives_shared_build_and_test_requirements(self):
        previous = Path.cwd()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root/'pyproject.toml').write_text('[project]\ndependencies=["packaging==24.2"]\n[build-system]\nrequires=["packaging==24.2"]\n[dependency-groups]\ndev=["packaging==24.2"]\n')
            try:
                os.chdir(root)
                _, purposes = adapter.requirement_lines()
            finally:
                os.chdir(previous)
            self.assertEqual(purposes['packaging'], 'runtime')

    def test_test_reporters_are_prepared_without_directory_guesses(self):
        previous = Path.cwd()
        with tempfile.TemporaryDirectory() as temporary:
            try:
                os.chdir(temporary)
                requirements, purposes = adapter.requirement_lines()
                self.assertIn('pytest==8.3.5', requirements)
                self.assertIn('pytest-cov==6.0.0', requirements)
                self.assertEqual(purposes['pytest'], 'test')
                self.assertEqual(purposes['pytest-cov'], 'test')
                requirements, _ = adapter.requirement_lines(['pytest==8.3.4','pytest-cov==5.0.0'])
                self.assertFalse(any(item.startswith(('pytest==','pytest-cov==')) for item in requirements))
            finally:
                os.chdir(previous)

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
