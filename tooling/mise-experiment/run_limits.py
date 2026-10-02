"""Verify actual production HTTP request/response/session limits."""
import argparse
from collections import Counter
import hashlib
import http.server
import json
from pathlib import Path
import platform
import subprocess
import threading


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    requests = Counter()

    class Origin(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def do_GET(self):
            requests[self.path] += 1
            if self.path == "/allowed/redirect":
                self.send_response(302)
                self.send_header("Location", "/allowed/redirect")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            size = int(self.path.rsplit("/", 1)[1]) if self.path.startswith("/allowed/bytes/") else 0
            self.send_response(200)
            self.send_header("Content-Length", str(size))
            self.end_headers()
            chunk = b"x" * (1024 * 1024)
            while size:
                piece = chunk[:min(size, len(chunk))]
                self.wfile.write(piece)
                size -= len(piece)

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Origin)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        result = subprocess.run([str(args.binary.resolve()), f"http://127.0.0.1:{server.server_port}/allowed/"],
                                text=True, capture_output=True, timeout=600)
        evidence = {"host": platform.platform(), "exit": result.returncode, "stderr": result.stderr,
                    "origin_requests": dict(requests), "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest()}
        here = Path(__file__).resolve().parent
        repository = here.parents[1]
        evidence["source_lf_sha256"] = {str(path.relative_to(repository)).replace("\\", "/"):
                                        hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest()
                                        for path in (repository / "src/broker.rs", here / "broker-host/src/bin/limits.rs", here / "run_limits.py")}
        evidence["production_commit"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repository, text=True).strip()
        if result.returncode == 0:
            evidence["cases"] = json.loads(result.stdout)
            assert requests["/allowed/empty"] == 4096
            assert requests["/allowed/redirect"] == 5
            assert requests["/allowed/bytes/134217728"] == 9
            assert requests["/allowed/bytes/134217729"] == 1
            assert requests["/allowed/bytes/1"] == 1
        args.output.write_text(json.dumps(evidence, indent=2) + "\n")
        print(json.dumps(evidence, indent=2))
        if result.returncode:
            raise SystemExit(result.returncode)
    finally:
        server.shutdown()


if __name__ == "__main__":
    main()
