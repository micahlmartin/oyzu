import unittest
from greeting import greet


class GreetingTests(unittest.TestCase):
    def test_named_greeting(self):
        self.assertEqual(greet("Oyzu"), "Hello, Oyzu!")

    def test_default(self):
        self.assertEqual(greet(), "Hello, world!")
