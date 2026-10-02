"""Credential-free HTTP adaptation to the production broker spool protocol.

Routes are host-supplied public source locations, never authorization material.
The production broker independently authorizes every request and redirect.
"""
import http.server
import threading
from urllib.parse import urlsplit

from broker_transport import fetch


def start(routes):
    class Handler(http.server.BaseHTTPRequestHandler):
        def relay(self, head=False):
            self.server.requests.append(self.path)
            path = urlsplit(self.path)
            route, separator, relative = path.path.lstrip("/").partition("/")
            if not separator or route not in routes or path.query or path.fragment:
                self.send_error(403)
                return
            info, body = fetch(routes[route] + relative)
            self.send_response(info["status"])
            self.send_header("Content-Type", info["contentType"])
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            if not head:
                self.wfile.write(body)

        def do_GET(self):
            self.relay()

        def do_HEAD(self):
            # The production channel currently implements GET. Retain its body
            # length as HEAD metadata; do not invent a second acquisition path.
            self.relay(head=True)

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.requests = []
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, f"http://127.0.0.1:{server.server_port}"
