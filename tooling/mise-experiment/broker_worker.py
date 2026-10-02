"""Run real mise acquisition and attacks inside the actual Oyzu executor."""
import json
import os
from pathlib import Path
import socket
import subprocess
import tomllib
import errno
import shutil

from broker_bridge import start
from broker_transport import fetch


def main():
    config = json.loads(Path("/workspace/worker.json").read_text())
    server, bridge = start(config["routes"])
    binary = "/usr/local/bin/oyzu-mise-spike"
    project = Path("/workspace/project")
    project.mkdir()
    (project / "oyzu.toml").write_text('[tools]\nnode = "22"\n')
    Path("/tmp/qualification-home").mkdir()
    environment = dict(os.environ, HOME="/tmp/qualification-home", XDG_CACHE_HOME="/tmp/qualification-cache",
                       OYZU_SPIKE_STATE="/tmp/mise-state", OYZU_SPIKE_ROUTE=bridge + "/node")
    results = []

    def case(name, fn):
        try:
            detail = fn()
        finally:
            Path("/out/bridge-requests.json").write_text(json.dumps(server.responses, indent=2))
        results.append({"case": name, "status": "passed", "detail": detail})
        Path("/out/worker-results.json").write_text(json.dumps(results, indent=2))

    def call(*args, code=0):
        result = subprocess.run([binary, *args], cwd=project, env=environment, capture_output=True, text=True, timeout=240)
        assert result.returncode == code, (args, result.returncode, result.stdout, result.stderr)
        return result.stdout.strip()

    def lock():
        assert call("lock") == "22.14.0"
        locked = tomllib.loads((project / "oyzu.lock").read_text())
        assert locked["tool"][0]["distribution"][0]["digest"] == "sha256:" + config["sha256"]
        return "backend metadata became an Oyzu lock through the production broker"

    case("broker-backend-lock", lock)
    original_lock = (project / "oyzu.lock").read_bytes()
    case("broker-real-node-install", lambda: json.loads(call("install")))

    def execute():
        before = len(server.requests)
        assert call("exec", "--", "node", "--version") == "v22.14.0"
        assert len(server.requests) == before, "frozen execution acquired content"
        assert (project / "oyzu.lock").read_bytes() == original_lock
        return "real locked Node executed; lock unchanged"
    case("broker-frozen-execution", execute)

    def blocked_raw():
        with socket.socket() as sock:
            sock.settimeout(2)
            try:
                sock.connect((config["egress_ip"], config["egress_port"]))
            except OSError:
                return "raw TCP to the positively tested origin is denied"
        raise AssertionError("raw egress succeeded")
    case("raw-ip-egress-denied", blocked_raw)

    def blocked_child(proxy):
        args = ["curl", "--silent", "--show-error", "--max-time", "3"]
        target = f'http://{config["egress_ip"]}:{config["egress_port"]}/public-control'
        args += ["--proxy", target] if proxy else ["--noproxy", "*"]
        result = subprocess.run([*args, target], capture_output=True, timeout=5)
        assert result.returncode != 0, "child acquired directly"
        return {"exit": result.returncode, "proxy_attempt": proxy}
    case("spawned-client-egress-denied", lambda: blocked_child(False))
    case("proxy-egress-denied", lambda: blocked_child(True))

    def blocked_dns():
        # Exercise the UDP path directly; a lookup failure alone could just be
        # an absent DNS record. This worker has no route to the control host.
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
            try:
                sock.sendto(b"\x00" * 12, (config["egress_ip"], 53))
            except OSError as error:
                assert error.errno in (errno.ENETUNREACH, errno.EHOSTUNREACH, errno.EPERM)
                return {"errno": error.errno, "protocol": "UDP", "port": 53}
        raise AssertionError("external DNS datagram escaped")
    case("dns-egress-denied", blocked_dns)

    def blocked_selected_tool():
        script = ("const net=require('net'); const s=net.connect(" +
                  str(config["egress_port"]) + "," + json.dumps(config["egress_ip"]) +
                  ");s.on('connect',()=>process.exit(2));"
                  "s.on('error',e=>{console.log(e.code);process.exit(0)});"
                  "setTimeout(()=>process.exit(3),3000)")
        assert call("exec", "--", "node", "-e", script) in ("ENETUNREACH", "EHOSTUNREACH", "EPERM")
        return "the backend-installed tool also cannot open a direct socket"
    case("installed-tool-egress-denied", blocked_selected_tool)

    def broker_denial(path):
        info, body = fetch(config["origin"] + path)
        assert info["status"] == 403, (info, body)
        return {"status": info["status"], "source": info["sourceId"]}
    case("broker-unapproved-source-denied", lambda: broker_denial("forbidden/artifact"))
    case("broker-redirect-reauthorized", lambda: broker_denial("approved/node/redirect"))
    for encoded in ("%2e%2e/forbidden", "%252e%252e/forbidden", "%2fforbidden", "%5cforbidden"):
        case("broker-encoded-path-denied-" + encoded.split("/")[0],
             lambda encoded=encoded: broker_denial("approved/node/" + encoded))

    def rejected_install(route):
        isolated = dict(environment, OYZU_SPIKE_STATE="/tmp/mise-" + route,
                        OYZU_SPIKE_ROUTE=bridge + "/" + route)
        result = subprocess.run([binary, "install"], cwd=project, env=isolated,
                                capture_output=True, text=True, timeout=240)
        assert result.returncode != 0, "invalid acquisition succeeded"
        if route == "tampered":
            assert "checksum" in result.stderr.lower(), result.stderr
        assert (project / "oyzu.lock").read_bytes() == original_lock
        execution = subprocess.run([binary, "exec", "--", "node", "--version"], cwd=project,
                                   env=isolated, capture_output=True, text=True, timeout=30)
        assert execution.returncode != 0, "failed install became executable"
        return {"install_exit": result.returncode, "execution_exit": execution.returncode,
                "lock_unchanged": True}
    case("broker-corrupt-archive-rejected", lambda: rejected_install("tampered"))
    case("broker-unavailable-source-rejected", lambda: rejected_install("unavailable"))

    def go_install():
        nonlocal project, environment
        previous_project, previous_environment = project, environment
        project = Path("/workspace/go-project")
        project.mkdir()
        go = config["go"]
        (project / "oyzu.toml").write_text('[tools]\ngo = "1.24.1"\n')
        # Independent official release metadata supplies a frozen Oyzu lock.
        # This case deliberately does not claim backend version discovery.
        lock = original_lock.decode().replace('core:node', 'core:go').replace('request = "22"', 'request = "1.24.1"')
        lock = lock.replace('22.14.0', '1.24.1').replace(config["sha256"], go["sha256"])
        node_size = tomllib.loads(original_lock.decode())["tool"][0]["distribution"][0]["size"]
        lock = lock.replace(f'size = {node_size}', f'size = {go["size"]}').replace('node-fixture', 'go-fixture')
        (project / "oyzu.lock").write_text(lock)
        artifacts = project / "artifact-routes.json"
        artifacts.write_text(json.dumps({"go-fixture:sha256:" + go["sha256"]: bridge + "/go/" + go["filename"]}))
        # These are separate one-tool acquisition cases, not a shared-store
        # capacity test. Release the completed Node fixture before using the
        # worker's bounded scratch store for Go.
        shutil.rmtree("/tmp/mise-state")
        environment = dict(previous_environment, OYZU_SPIKE_STATE="/tmp/mise-go",
                           OYZU_SPIKE_ROUTE=bridge + "/go", OYZU_SPIKE_ARTIFACTS=str(artifacts))
        try:
            call("install")
            assert call("exec", "--", "go", "version") == "go version go1.24.1 linux/amd64"
            before = len(server.requests)
            assert call("exec", "--", "go", "env", "GOROOT").endswith("/go/1.24.1")
            assert len(server.requests) == before
            assert (project / "oyzu.lock").read_text() == lock
            return "real core Go archive installed through broker; frozen execution and GOROOT verified"
        finally:
            project, environment = previous_project, previous_environment
    case("broker-core-go-frozen-install-exec", go_install)

    def java_install():
        nonlocal project, environment
        previous_project, previous_environment = project, environment
        project = Path("/workspace/java-project")
        project.mkdir()
        java = config["java"]
        version = java["locked_version"]
        (project / "oyzu.toml").write_text('[tools]\njava = ' + json.dumps(version) + '\n')
        lock = original_lock.decode().replace('core:node', 'core:java').replace('request = "22"', 'request = ' + json.dumps(version))
        lock = lock.replace('22.14.0', version).replace(config["sha256"], java["sha256"])
        node_size = tomllib.loads(original_lock.decode())["tool"][0]["distribution"][0]["size"]
        lock = lock.replace(f'size = {node_size}', f'size = {java["size"]}').replace('node-fixture', 'java-fixture')
        (project / "oyzu.lock").write_text(lock)
        artifacts = project / "artifact-routes.json"
        artifacts.write_text(json.dumps({"java-fixture:sha256:" + java["sha256"]: bridge + "/java/" + java["filename"]}))
        shutil.rmtree("/tmp/mise-go")
        environment = dict(previous_environment, OYZU_SPIKE_STATE="/tmp/mise-java",
                           OYZU_SPIKE_ROUTE=bridge + "/java", OYZU_SPIKE_ARTIFACTS=str(artifacts),
                           OYZU_SPIKE_URL_REPLACEMENTS=json.dumps({java["metadata_url"]: bridge + "/java/metadata.json"}))
        try:
            call("install")
            before = len(server.requests)
            result = subprocess.run([binary, "exec", "--", "java", "-version"], cwd=project,
                                    env=environment, capture_output=True, text=True, timeout=30)
            assert result.returncode == 0, result.stderr
            assert '1.8.0_442' in result.stderr, result.stderr
            assert len(server.requests) == before
            assert (project / "oyzu.lock").read_text() == lock
            assert any("/java/metadata.json" in request for request in server.requests)
            return "real Temurin JDK, metadata and archive through broker; frozen execution verified"
        finally:
            project, environment = previous_project, previous_environment
    case("broker-core-java-frozen-install-exec", java_install)

    def python_install(invalid=False):
        nonlocal project, environment
        previous_project, previous_environment = project, environment
        project = Path("/workspace/python-invalid" if invalid else "/workspace/python-project")
        project.mkdir()
        python = config["python"]
        version = python["version"]
        (project / "oyzu.toml").write_text('[tools]\npython = ' + json.dumps(version) + '\n')
        lock = original_lock.decode().replace('core:node', 'core:python').replace('request = "22"', 'request = ' + json.dumps(version))
        lock = lock.replace('22.14.0', version).replace(config["sha256"], python["sha256"])
        node_size = tomllib.loads(original_lock.decode())["tool"][0]["distribution"][0]["size"]
        lock = lock.replace(f'size = {node_size}', f'size = {python["size"]}').replace('node-fixture', 'python-fixture')
        (project / "oyzu.lock").write_text(lock)
        artifacts = project / "artifact-routes.json"
        artifacts.write_text(json.dumps({"python-fixture:sha256:" + python["sha256"]: bridge + "/python/" + python["filename"]}))
        shutil.rmtree("/tmp/mise-python" if invalid else "/tmp/mise-java")
        api_route = "/python/invalid-api" if invalid else "/python/api"
        environment = dict(previous_environment, OYZU_SPIKE_STATE="/tmp/mise-python",
                           OYZU_SPIKE_ROUTE=bridge + "/python", OYZU_SPIKE_ARTIFACTS=str(artifacts),
                           OYZU_SPIKE_URL_REPLACEMENTS=json.dumps({"https://api.github.com": bridge + api_route,
                               "https://tuf-repo-cdn.sigstore.dev": bridge + "/tuf"}))
        try:
            if invalid:
                result = subprocess.run([binary, "install"], cwd=project, env=environment,
                                        capture_output=True, text=True, timeout=240)
                assert result.returncode != 0, "invalid signature was accepted"
                assert "attestation" in result.stderr.lower(), result.stderr
                assert (project / "oyzu.lock").read_text() == lock
                return {"exit": result.returncode, "diagnostic": result.stderr.strip()}
            call("install")
            before = len(server.requests)
            assert call("exec", "--", "python", "--version") == "Python 3.12.9"
            assert len(server.requests) == before
            assert (project / "oyzu.lock").read_text() == lock
            assert any("/python/api/" in request for request in server.requests)
            return "real CPython archive and GitHub attestations through broker; verification enabled; frozen execution verified"
        finally:
            project, environment = previous_project, previous_environment
    case("broker-core-python-provenance-install-exec", python_install)
    case("broker-core-python-invalid-attestation-denied", lambda: python_install(True))

    def aqua_install():
        nonlocal project, environment
        previous_project, previous_environment = project, environment
        project = Path("/workspace/jq-project")
        project.mkdir()
        jq = config["jq"]
        (project / "oyzu.toml").write_text('[tools]\njq = "1.7.1"\n')
        lock = original_lock.decode().replace('core:node', 'aqua:jqlang/jq').replace('request = "22"', 'request = "1.7.1"')
        lock = lock.replace('22.14.0', '1.7.1').replace(config["sha256"], jq["sha256"])
        node_size = tomllib.loads(original_lock.decode())["tool"][0]["distribution"][0]["size"]
        lock = lock.replace(f'size = {node_size}', f'size = {jq["size"]}').replace('node-fixture', 'jq-fixture')
        (project / "oyzu.lock").write_text(lock)
        artifacts = project / "artifact-routes.json"
        artifacts.write_text(json.dumps({"jq-fixture:sha256:" + jq["sha256"]: bridge + "/jq/jq-linux-amd64"}))
        environment = dict(previous_environment, OYZU_SPIKE_STATE="/tmp/mise-jq",
                           OYZU_SPIKE_ROUTE=bridge + "/jq", OYZU_SPIKE_ARTIFACTS=str(artifacts),
                           OYZU_SPIKE_BACKENDS=json.dumps({"jq": "aqua:jqlang/jq"}),
                           OYZU_SPIKE_URL_REPLACEMENTS=json.dumps({"https://api.github.com": bridge + "/jq/api",
                               "https://github.com/jqlang/jq/releases/download": bridge + "/jq/release"}))
        try:
            call("install")
            before = len(server.requests)
            assert call("exec", "--", "jq", "--version") == "jq-1.7.1"
            assert call("exec", "--", "jq", "-n", "{answer: (6 * 7)}") == '{\n  "answer": 42\n}'
            assert len(server.requests) == before
            assert (project / "oyzu.lock").read_text() == lock
            return "real aqua:jqlang/jq backend with baked pinned registry; broker acquisition and frozen execution"
        finally:
            project, environment = previous_project, previous_environment
    case("broker-aqua-jq-install-exec", aqua_install)

    def credential_error():
        info, body = fetch(config["origin"] + "approved/node/error")
        assert info["status"] == 401
        assert body == b"approved source returned HTTP 401"
        assert set(info) == {"status", "sourceId", "contentType"}
        return "upstream error body and headers were not relayed"
    case("broker-error-sanitization", credential_error)

    def isolation():
        status = dict(line.split(":", 1) for line in Path("/proc/self/status").read_text().splitlines() if ":" in line)
        assert int(status["CapEff"].strip(), 16) == 0
        assert status["NoNewPrivs"].strip() == "1"
        assert sorted(p.name for p in Path("/sys/class/net").iterdir()) == ["lo"]
        try:
            Path("/usr/local/oyzu-write-probe").write_text("must be denied")
        except OSError:
            pass
        else:
            raise AssertionError("root filesystem writable")
        assert not Path("/var/run/docker.sock").exists()
        # Host examines these values for its private canaries, without telling
        # the untrusted worker what those canaries are.
        Path("/out/worker-environment.json").write_text(json.dumps(dict(os.environ)))
        return "network none, read-only root, no capabilities, no-new-privileges, no Docker socket"
    case("production-worker-isolation", isolation)
    server.shutdown()
    print(json.dumps({"cases": len(results), "status": "passed"}))


if __name__ == "__main__":
    main()
