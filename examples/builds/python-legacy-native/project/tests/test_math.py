import unittest
from fallback import add


class MathTests(unittest.TestCase):
    def test_reference_behavior(self):
        self.assertEqual(add(2, 3), 5)

    def test_native_behavior(self):
        import native_math

        self.assertEqual(native_math.add(2, 3), 5)
