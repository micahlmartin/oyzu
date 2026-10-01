import json
from wsgiref.simple_server import make_server

def app(environ, start_response):
    found = environ.get("PATH_INFO") == "/health"
    body = json.dumps({"status": "ok"} if found else {"error": "not found"}).encode()
    start_response("200 OK" if found else "404 Not Found", [("Content-Type", "application/json")])
    return [body]

def main():
    with make_server("127.0.0.1", 8080, app) as server:
        server.serve_forever()
