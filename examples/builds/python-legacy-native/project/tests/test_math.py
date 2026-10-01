import unittest
from fallback import add

class MathTests(unittest.TestCase):
    def test_reference_behavior(self):
        self.assertEqual(add(2, 3), 5)
