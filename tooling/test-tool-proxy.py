"""Install and execute real Node through an authenticated local forwarding proxy."""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from threading import Thread
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--disposable-linux-host", action="store_true",
                        help="Authorize temporary /etc/oyzu administrative policy on a disposable Linux host")
    args = parser.parse_args()
    if not args.disposable_linux_host or sys.platform != "linux" or os.geteuid() != 0:
        parser.error("run as root on a disposable Linux host with --disposable-linux-host")
    cli = args.cli.resolve(strict=True)
    requests = []
    # Synthetic acceptance credential, never a real corporate secret.
    authorization = "Bearer oyzu-local-proxy-acceptance"

    class Proxy(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_GET(self):
            requests.append(self.path)
            if self.headers.get("Authorization") != authorization:
                self.send_error(403)
                return
            if not self.path.startswith("/node/"):
                self.send_error(404)
                return
            try:
                # Forward actual upstream bytes; do not manufacture a catalog,
                # checksum or executable. The auth header stays at this proxy.
                with urllib.request.urlopen("https://nodejs.org/dist/" + self.path[6:], timeout=60) as upstream:
                    body = upstream.read()
                    self.send_response(upstream.status)
                    self.send_header("Content-Type", upstream.headers.get("Content-Type", "application/octet-stream"))
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
            except Exception:
                self.send_error(502)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    admin = Path("/etc/oyzu/admin-settings.json")
    admin.parent.mkdir(mode=0o755, exist_ok=True)
    policy = {"schemaVersion": 1, "kind": "local-admin-policy", "profiles": {}, "requiredCapabilities": [],
              "settings": {"registries.routes": {"locked": True, "value": [
                  {"protocol": "tools", "scope": "core:node", "connectorId": "corp-node"}]}}}
    # Refuse to overwrite any existing policy, even on the disposable host.
    with admin.open("x", encoding="utf-8") as output:
        json.dump(policy, output)
    try:
        with tempfile.TemporaryDirectory(prefix="oyzu-proxy-acceptance-") as temporary:
            root = Path(temporary)
            project = root / "project"
            project.mkdir()
            (project / "oyzu.toml").write_text(
                '[tools]\nnode = "22.15.0"\n',
                encoding="utf-8",
            )
            binding = root / "host-bindings.toml"
            binding.write_text(
                f'format = 1\n[connectors.corp-node]\nbase_url = "http://127.0.0.1:{server.server_port}/node/"\n'
                'authorization_env = "OYZU_ACCEPTANCE_PROXY_AUTH"\n', encoding="utf-8",
            )
            environment = dict(os.environ, OYZU_ACCEPTANCE_PROXY_AUTH=authorization)

            def run(command, expected=0, env=environment):
                result = subprocess.run([str(cli), "-C", str(project)] + command,
                                        env=env, capture_output=True, text=True, timeout=300)
                assert authorization not in result.stdout + result.stderr
                if result.returncode != expected:
                    raise RuntimeError(f"{command}: exit {result.returncode}\n{result.stdout}\n{result.stderr}")
                return result.stdout.strip() if expected == 0 else result.stderr

            install = ["install", "--connector-bindings", str(binding)]
            run(["install"], expected=2)
            assert not (project / "oyzu.lock").exists()
            assert not requests
            denied = run(install, expected=2, env=dict(environment, OYZU_ACCEPTANCE_PROXY_AUTH="denied"))
            assert requests and not (project / "oyzu.lock").exists(), denied
            requests.clear()
            run(install)
            assert any(path.endswith("index.json") for path in requests), requests
            assert any(path.endswith("SHASUMS256.txt") for path in requests), requests
            assert any(path.endswith((".tar.gz", ".tar.xz", ".zip")) for path in requests), requests
            lock = (project / "oyzu.lock").read_bytes()
            assert authorization.encode() not in lock and b"127.0.0.1" not in lock
            assert run(["exec", "--", "node", "--version"]) == "v22.15.0"
            count = len(requests)
            run(["install", "--frozen", "--offline"], env=dict(os.environ))
            assert len(requests) == count
            assert (project / "oyzu.lock").read_bytes() == lock
            print(json.dumps({"verified": ["real catalog, checksum and archive through authenticated proxy",
                                           "missing binding and denied authorization fail without public fallback",
                                           "installed Node executes", "frozen offline reuse needs no host credentials"],
                              "scope": "standalone explicit host bindings; managed agent bindings remain separate"}, indent=2))
    finally:
        admin.unlink()
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
