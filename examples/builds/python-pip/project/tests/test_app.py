import unittest
from app import health, supported_version


class AppTests(unittest.TestCase):
    def test_health(self):
        self.assertEqual(health(), {"status": "ok"})

    def test_version_constraint(self):
        self.assertTrue(supported_version("1.2"))
        self.assertFalse(supported_version("1.0rc1"))
