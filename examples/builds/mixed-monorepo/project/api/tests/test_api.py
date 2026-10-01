import json
import unittest
from api import app

class ApiTests(unittest.TestCase):
    def call(self, path):
        status = []
        body = b"".join(app({"PATH_INFO": path}, lambda value, headers: status.append(value)))
        return status[0], json.loads(body)
    def test_health(self):
        self.assertEqual(self.call("/health"), ("200 OK", {"status": "ok"}))
    def test_missing(self):
        self.assertEqual(self.call("/missing")[0], "404 Not Found")
