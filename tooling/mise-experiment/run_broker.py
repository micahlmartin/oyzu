"""Qualify real Oyzu broker/executor acquisition using real backend artifacts."""
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import subprocess
import threading
import time
import uuid
from urllib.parse import urlsplit


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--image", default="oyzu-mise-broker-qualification:local")
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    repository = here.parents[1]
    def canonical_hash(path):
        return hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest()
    source_files = [repository / "Cargo.toml", repository / "Cargo.lock", *sorted((repository / "src").rglob("*"))]
    identity = {
        "production_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repository, text=True).strip(),
        "production_source_lf_sha256": {str(p.relative_to(repository)).replace("\\", "/"): canonical_hash(p)
                                         for p in source_files if p.is_file()},
        "harness_lf_sha256": {p.name: canonical_hash(p) for p in
                              [here / name for name in ("run_broker.py", "broker_worker.py", "broker_bridge.py", "spike.rs", "embedding.patch")]},
        "driver_sha256": hashlib.sha256(args.driver.read_bytes()).hexdigest(),
        "image": json.loads(subprocess.check_output(["docker", "image", "inspect", args.image], text=True))[0]["Id"],
    }
    root = args.work.resolve() / f"broker-run-{time.time_ns()}"
    for name in ("workspace", "output", "spool", "private"):
        (root / name).mkdir(parents=True)
    canary = "oyzu-test-only-" + uuid.uuid4().hex
    inventory = json.loads((args.work / "archives/inventory-linux-x64.json").read_text())
    item = inventory["22.14.0"]
    archive = args.work / "archives" / item["name"]
    assert hashlib.sha256(archive.read_bytes()).hexdigest() == item["sha256"]
    backend_inventory = json.loads((args.work / "backend-archives/inventory-linux-x64.json").read_text())
    go = backend_inventory["go"]
    go_archive = args.work / "backend-archives" / go["filename"]
    assert hashlib.sha256(go_archive.read_bytes()).hexdigest() == go["sha256"]
    java = backend_inventory["java"]
    java_archive = args.work / "backend-archives" / java["filename"]
    java_metadata = (args.work / "backend-archives/java-linux-x64.json").read_bytes()
    assert hashlib.sha256(java_archive.read_bytes()).hexdigest() == java["sha256"]
    assert hashlib.sha256(java_metadata).hexdigest() == java["metadata_sha256"]
    python = backend_inventory["python"]
    python_archive = args.work / "backend-archives" / python["filename"]
    attestations = (args.work / "backend-archives/python-attestations.json").read_bytes()
    assert hashlib.sha256(python_archive.read_bytes()).hexdigest() == python["sha256"]
    assert hashlib.sha256(attestations).hexdigest() == python["attestation_sha256"]
    invalid = json.loads(attestations)
    for attestation in invalid["attestations"]:
        for signature in attestation["bundle"]["dsseEnvelope"]["signatures"]:
            sig = signature["sig"]
            signature["sig"] = ("A" if sig[0] != "A" else "B") + sig[1:]
    invalid_attestations = json.dumps(invalid).encode()
    jq = backend_inventory["jq"]
    jq_archive = args.work / "backend-archives" / jq["filename"]
    jq_release = (args.work / "backend-archives/jq-release.json").read_bytes()
    jq_checksums = (args.work / "backend-archives/jq-sha256sum.txt").read_bytes()
    assert hashlib.sha256(jq_archive.read_bytes()).hexdigest() == jq["sha256"]
    assert hashlib.sha256(jq_release).hexdigest() == jq["release_sha256"]
    assert hashlib.sha256(jq_checksums).hexdigest() == jq["checksums_sha256"]
    requests = []

    class Origin(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            authorized = self.headers.get("Authorization") == "Bearer " + canary
            requests.append({"path": self.path, "authorized": authorized})
            if self.path == "/public-control":
                status, body = 200, b"reachable-origin"
            elif not authorized:
                status, body = 401, canary.encode()
            elif self.path == "/approved/node/redirect":
                self.send_response(302)
                self.send_header("Location", f"http://127.0.0.1:{self.server.server_port}/forbidden/artifact")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            elif self.path == "/approved/node/error":
                status, body = 401, canary.encode()
            elif self.path.startswith("/approved/node/tampered/"):
                status, body = 200, b"deliberately corrupted qualification archive"
            elif self.path.startswith("/approved/node/unavailable/"):
                status, body = 503, canary.encode()
            elif self.path == "/approved/node/index.json":
                status, body = 200, json.dumps([{"version": "v22.14.0", "date": "2025-02-11", "files": []}]).encode()
            elif self.path == "/approved/node/v22.14.0/SHASUMS256.txt":
                status, body = 200, f'{item["sha256"]}  {item["name"]}\n'.encode()
            elif self.path == "/approved/node/v22.14.0/" + item["name"]:
                status, body = 200, archive.read_bytes()
            elif self.path == "/approved/go/" + go["filename"]:
                status, body = 200, go_archive.read_bytes()
            elif self.path == "/approved/go/" + go["filename"] + ".sha256":
                status, body = 200, go["sha256"].encode()
            elif self.path == "/approved/java/" + java["filename"]:
                status, body = 200, java_archive.read_bytes()
            elif self.path == "/approved/java/metadata.json":
                status, body = 200, java_metadata
            elif self.path == "/approved/python/" + python["filename"]:
                status, body = 200, python_archive.read_bytes()
            elif urlsplit(self.path).path == "/approved/python/api/repos/astral-sh/python-build-standalone/attestations/sha256:" + python["sha256"]:
                status, body = 200, attestations
            elif urlsplit(self.path).path == "/approved/python/invalid-api/repos/astral-sh/python-build-standalone/attestations/sha256:" + python["sha256"]:
                status, body = 200, invalid_attestations
            elif self.path in ("/approved/jq/jq-linux-amd64", "/approved/jq/release/jq-1.7.1/jq-linux-amd64"):
                status, body = 200, jq_archive.read_bytes()
            elif self.path == "/approved/jq/release/jq-1.7.1/sha256sum.txt":
                status, body = 200, jq_checksums
            elif self.path == "/approved/jq/api/repos/jqlang/jq/releases/tags/jq-1.7.1":
                status, body = 200, jq_release
            else:
                status, body = 404, b"not found"
            self.send_response(status)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Content-Type", "application/octet-stream")
            self.send_header("Set-Cookie", "test=" + canary)
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("0.0.0.0", 0), Origin)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    origin = f"http://127.0.0.1:{server.server_port}/"
    try:
        # Positive control: the very same origin is reachable outside the enforced
        # worker. A failed public Internet request alone would not prove isolation.
        probe = ("import json,socket,urllib.request; "
                 "ip=socket.gethostbyname('host.docker.internal'); "
                 f"u='http://'+ip+':{server.server_port}/public-control'; "
                 "b=urllib.request.build_opener(urllib.request.ProxyHandler({})).open(u,timeout=5).read(); "
                 "assert b==b'reachable-origin'; print(json.dumps({'ip':ip,'reachable':True}))")
        positive = json.loads(subprocess.check_output([
            "docker", "run", "--rm", "--pull=never", "--entrypoint", "python3", args.image, "-c", probe,
        ], text=True, timeout=30))
        worker = {"origin": origin, "routes": {"node": origin + "approved/node/",
                  "tampered": origin + "approved/node/tampered/",
                  "unavailable": origin + "approved/node/unavailable/",
                  "go": origin + "approved/go/", "java": origin + "approved/java/",
                  "python": origin + "approved/python/", "tuf": "https://tuf-repo-cdn.sigstore.dev/",
                  "jq": origin + "approved/jq/"},
                  "go": go, "java": java, "python": python, "jq": jq,
                  "sha256": item["sha256"], "egress_ip": positive["ip"], "egress_port": server.server_port}
        (root / "workspace/worker.json").write_text(json.dumps(worker))
        configuration = {
            "image": args.image, "workspace": str(root / "workspace"), "output": str(root / "output"),
            "spool": str(root / "spool"), "private": str(root / "private"),
            "sources": [{"id": "node-fixture", "base": origin + "approved/node/", "authorization": "Bearer " + canary},
                        {"id": "go-fixture", "base": origin + "approved/go/", "authorization": "Bearer " + canary},
                        {"id": "java-fixture", "base": origin + "approved/java/", "authorization": "Bearer " + canary},
                        {"id": "python-fixture", "base": origin + "approved/python/", "authorization": "Bearer " + canary},
                        {"id": "jq-fixture", "base": origin + "approved/jq/", "authorization": "Bearer " + canary},
                        {"id": "sigstore-public-tuf", "base": "https://tuf-repo-cdn.sigstore.dev/", "authorization": None}],
            "argv": ["python3", "/opt/qualification/broker_worker.py"],
            "env": {"PYTHONDONTWRITEBYTECODE": "1"}, "timeout_seconds": 600,
            "name": "oyzu-broker-" + uuid.uuid4().hex[:12],
        }
        config_path = root / "private/session.json"
        config_path.write_text(json.dumps(configuration))
        (root / "private/credential-canary.txt").write_text(canary)
        result = subprocess.run([str(args.driver.resolve()), str(config_path)], capture_output=True, text=True,
                                env=dict(os.environ, OYZU_HOST_ONLY_CANARY=canary), timeout=660)
        evidence = {"host": os.name, "positive_control": positive, "driver_exit": result.returncode,
                    "driver_stdout": result.stdout, "driver_stderr": result.stderr, "origin_requests": requests}
        evidence["identity"] = identity
        evidence["backend_artifacts"] = backend_inventory
        evidence["worker_stderr"] = (root / "output/stderr.log").read_text() if (root / "output/stderr.log").exists() else None
        if (root / "output/bridge-requests.json").exists():
            evidence["bridge_requests"] = json.loads((root / "output/bridge-requests.json").read_text())
        result_file = root / "output/worker-results.json"
        if result_file.exists():
            evidence["cases"] = json.loads(result_file.read_text())
        assert canary not in json.dumps(evidence), "host canary leaked in report"
        for directory in (root / "output", root / "spool", root / "workspace"):
            for file in directory.rglob("*"):
                if file.is_file():
                    assert canary.encode() not in file.read_bytes(), "host canary reached worker-visible content"
        evidence["credential_canary_absent"] = True
        assert not any(r["path"].startswith("/forbidden") for r in requests), "denied origin was contacted"
        assert all(r["authorized"] for r in requests if r["path"].startswith("/approved/"))
        evidence["source_credentials_checked"] = True
        (root / "results.json").write_text(json.dumps(evidence, indent=2))
        print(f"Evidence: {root / 'results.json'}", flush=True)
        if result.returncode:
            print((root / "output/stderr.log").read_text(), flush=True)
            raise SystemExit(result.returncode)
    finally:
        server.shutdown()


if __name__ == "__main__":
    main()
