"""Exercise provisioning retries against a real loopback HTTP server."""
from collections import Counter
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import threading
import unittest
from unittest.mock import patch
import urllib.error

from provisioning import download


class Downloads(unittest.TestCase):
    def test_transient_failures_limits_and_terminal_errors(self):
        calls = Counter()

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                calls[self.path] += 1
                count = calls[self.path]
                status = 200
                if self.path == '/recover' and count < 3:
                    status = 500
                if self.path == '/denied':
                    status = 403
                if self.path == '/unavailable':
                    status = 503
                self.send_response(status)
                self.send_header('Content-Length', '100' if self.path == '/partial' and count == 1 else '5')
                self.end_headers()
                self.wfile.write(b'bytes')

            def log_message(self, *args):
                pass

        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        base = f'http://127.0.0.1:{server.server_port}'
        try:
            with patch('provisioning.time.sleep') as sleep:
                self.assertEqual(download(base+'/recover', 5), b'bytes')
                self.assertEqual([call.args[0] for call in sleep.call_args_list], [1, 2])
                self.assertEqual(calls['/recover'], 3)
                self.assertEqual(download(base+'/partial', 128), b'bytes')
                self.assertEqual(calls['/partial'], 2)
                with self.assertRaises(urllib.error.HTTPError):
                    download(base+'/denied', 5)
                self.assertEqual(calls['/denied'], 1)
                with self.assertRaises(urllib.error.HTTPError):
                    download(base+'/unavailable', 5)
                self.assertEqual(calls['/unavailable'], 3)
                with self.assertRaises(ValueError):
                    download(base+'/oversize', 4)
                self.assertEqual(calls['/oversize'], 1)
        finally:
            server.shutdown()
            server.server_close()
            thread.join()


if __name__ == '__main__':
    unittest.main()
