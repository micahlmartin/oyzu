#!/usr/bin/env python3
"""Exercise the linked upstream library using real Node archives and Oyzu files."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import http.server
import json
import os
from pathlib import Path
import platform
import queue
import shutil
import signal
import statistics
import subprocess
import threading
import time
import urllib.request

PIN = "da0db43e9398b46bafa95232014708a51120e731"
VERSIONS = ("22.14.0", "24.0.0")


def host():
    system = {"Windows": "win", "Linux": "linux", "Darwin": "darwin"}[platform.system()]
    arch = "arm64" if platform.machine().lower() in ("arm64", "aarch64") else "x64"
    return system, arch


def archive_name(version):
    system, arch = host()
    ext = "zip" if system == "win" else "tar.gz"
    return f"node-v{version}-{system}-{arch}.{ext}"


def provision(work):
    archives = work / "archives"
    archives.mkdir(parents=True, exist_ok=True)
    inventory = {}
    for version in VERSIONS:
        name = archive_name(version)
        base = f"https://nodejs.org/dist/v{version}/"
        sums = urllib.request.urlopen(base + "SHASUMS256.txt", timeout=60).read().decode()
        digest = next(line.split()[0] for line in sums.splitlines() if line.split()[-1] == name)
        archive = archives / name
        if not archive.exists() or hashlib.file_digest(archive.open("rb"), "sha256").hexdigest() != digest:
            print(f"Provisioning {name}", flush=True)
            with urllib.request.urlopen(base + name, timeout=120) as response, archive.open("wb") as out:
                shutil.copyfileobj(response, out)
        with archive.open("rb") as src:
            assert hashlib.file_digest(src, "sha256").hexdigest() == digest
        inventory[version] = {"name": name, "sha256": digest, "size": archive.stat().st_size}
    (archives / f"inventory-{host()[0]}-{host()[1]}.json").write_text(json.dumps(inventory, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--work", required=True, type=Path)
    parser.add_argument("--provision", action="store_true")
    parser.add_argument("--network-isolated", action="store_true")
    parser.add_argument("--windows-shim-mode", choices=("file", "hardlink"), default="file")
    args = parser.parse_args()
    work = args.work.resolve()
    work.mkdir(parents=True, exist_ok=True)
    if args.provision:
        provision(work)
        return
    if not args.binary:
        parser.error("--binary required unless provisioning")
    binary = args.binary.resolve()
    harness = Path(__file__).resolve().parent
    evidence = {
        "host": platform.platform(), "upstream": PIN,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "harness_sha256": {name: hashlib.sha256((harness / name).read_bytes()).hexdigest()
                           for name in ("spike.rs", "embedding.patch", "run.py", "shell-test.sh", "shell-test.ps1")},
        "harness_lf_sha256": {name: hashlib.sha256((harness / name).read_bytes().replace(b"\r\n", b"\n")).hexdigest()
                              for name in ("spike.rs", "embedding.patch", "run.py", "shell-test.sh", "shell-test.ps1")},
        "network_isolated": args.network_isolated,
        "windows_shim_mode": args.windows_shim_mode if os.name == "nt" else None,
        "build_profile": "dev (unoptimized)",
    }
    inventory = json.loads((work / "archives" / f"inventory-{host()[0]}-{host()[1]}.json").read_text())
    run_root = work / f"run-{time.time_ns()}"
    run_root.mkdir()
    requests = []
    results = []

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_HEAD(self):
            requests.append("HEAD " + self.path)
            source = work / "archives" / self.path.rsplit("/", 1)[-1]
            if not source.is_file():
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header("Content-Length", str(source.stat().st_size))
            self.end_headers()

        def do_GET(self):
            requests.append(self.path)
            if self.path.startswith("/redirect/"):
                self.send_response(302)
                self.send_header("Location", f"http://127.0.0.1:{self.server.server_port}/forbidden/blob")
                self.end_headers()
                return
            if self.path.startswith("/missing/"):
                self.send_error(404)
                return
            if self.path.endswith("index.json"):
                body = json.dumps([{"version": "v" + v, "date": "2025-01-01", "files": []} for v in reversed(VERSIONS)]).encode()
            elif self.path.endswith("SHASUMS256.txt"):
                body = "".join(f"{i['sha256']}  {i['name']}\n" for i in inventory.values()).encode()
            else:
                filename = self.path.rsplit("/", 1)[-1]
                source = work / "archives" / filename
                if not source.is_file() or filename not in [i["name"] for i in inventory.values()]:
                    self.send_error(404)
                    return
                if self.path.startswith("/corrupt/"):
                    body = b"invalid content"
                else:
                    body = source.read_bytes()
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            try:
                self.wfile.write(body)
            except (BrokenPipeError, ConnectionResetError):
                pass

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    route = f"http://127.0.0.1:{server.server_port}/ok"
    base_env = dict(os.environ, OYZU_SPIKE_STATE=str(run_root / "state"), OYZU_SPIKE_ROUTE=route)
    if os.name == "nt":
        base_env["OYZU_SPIKE_WINDOWS_SHIM_MODE"] = args.windows_shim_mode

    def call(*command, cwd=None, env=None, code=0):
        completed = subprocess.run([str(binary), *command], cwd=cwd or run_root, env=env or base_env,
                                   text=True, encoding="utf-8", capture_output=True, timeout=180)
        if completed.returncode != code:
            raise AssertionError(f"{command}: exit {completed.returncode}, expected {code}\n{completed.stdout}\n{completed.stderr}")
        return completed.stdout.strip()

    def denied(expected, *command, cwd=None, env=None):
        output = subprocess.run([str(binary), *command], cwd=cwd or run_root,
                                env=env or base_env, text=True, capture_output=True, timeout=180)
        assert output.returncode == 1, output.stdout + output.stderr
        assert expected.lower() in output.stderr.lower(), output.stderr
        return expected

    def record(name, fn):
        start = time.monotonic()
        try:
            detail = fn()
            result = {"case": name, "status": "passed", "detail": detail}
        except Exception as error:
            result = {"case": name, "status": "failed", "detail": str(error)}
        result["seconds"] = round(time.monotonic() - start, 3)
        results.append(result)
        print(json.dumps(result), flush=True)
        (run_root / "results.json").write_text(json.dumps({**evidence, "results": results, "requests": requests}, indent=2))

    # Get the actual upstream platform key rather than guessing its ABI spelling.
    identity = json.loads(call("inspect"))
    projects = []
    for version in VERSIONS:
        project = run_root / ("project-" + version)
        project.mkdir()
        request = version.split(".")[0]
        (project / "oyzu.toml").write_text(f'[tools]\nnode = "{request}"\n[env]\nOYZU_PROJECT = "{request}"\n')
        item = inventory[version]
        (project / "oyzu.lock").write_text(f'''format = 1
[[tool]]
id = "core:node"
request = "{request}"
version = "{version}"
backend_digest = "git:{PIN}"
[[tool.distribution]]
platform = "{identity['platform']}"
digest = "sha256:{item['sha256']}"
size = {item['size']}
source_id = "node-fixture"
verification = "fixture-sha256"
dependencies = []
''')
        projects.append(project)
    a, b = projects
    original_locks = [(p / "oyzu.lock").read_bytes() for p in projects]

    def equal(actual, expected):
        assert actual == expected, (actual, expected)
        return actual

    record("activation-does-not-install", lambda: denied("locked tool is not installed", "hook-env", "-s", "pwsh" if os.name == "nt" else "bash", cwd=a))
    record("backend-version-resolution", lambda: equal(call("resolve", cwd=a), VERSIONS[0]))
    def lock_creation():
        p = run_root / "lock-creation"
        p.mkdir()
        (p / "oyzu.toml").write_text('[tools]\nnode="22"\n')
        equal(call("lock", cwd=p), VERSIONS[0])
        import tomllib
        text = (p / "oyzu.lock").read_text()
        created = tomllib.loads(text)
        equal(created["tool"][0]["distribution"][0]["digest"], "sha256:" + inventory[VERSIONS[0]]["sha256"])
        assert "http" not in text and str(server.server_port) not in text
        equal(call("exec", "--", "node", "--version", cwd=p, code=1), "")
        return created
    record("backend-metadata-to-oyzu-lock", lock_creation)
    record("real-backend-install-a", lambda: json.loads(call("install", cwd=a)))
    record("real-backend-install-b", lambda: json.loads(call("install", cwd=b)))
    for p, version in zip(projects, VERSIONS):
        record(f"exec-{version}", lambda p=p, version=version: equal(call("exec", "--", "node", "--version", cwd=p), "v"+version))
    record("exec-arguments", lambda: equal(json.loads(call("exec", "--", "node", "-e", "console.log(JSON.stringify(process.argv.slice(1)))", "--", "a b", "single'quote", 'double"quote', "$literal", cwd=a)), ["a b", "single'quote", 'double"quote', "$literal"]))
    record("exec-exit-status", lambda: call("exec", "--", "node", "-e", "process.exit(37)", cwd=a, code=37))
    edge_arguments = ["", "caf\u00e9\u96ea", "C:\\ends\\", "literal &|<>^()%! text"]
    record("exec-argument-edge-cases", lambda: equal(json.loads(call("exec", "--", "node", "-e",
           "console.log(JSON.stringify(process.argv.slice(1)))", "--", *edge_arguments, "line\nbreak", cwd=a)),
           [*edge_arguments, "line\nbreak"]))

    def unusual_project_path():
        unusual = run_root / "project space caf\u00e9\u96ea"
        unusual.mkdir()
        for name in ("oyzu.toml", "oyzu.lock"):
            (unusual / name).write_bytes((a / name).read_bytes())
        equal(call("exec", "--", "node", "--version", cwd=unusual), "v" + VERSIONS[0])
        equal(json.loads(call("exec", "--", "node", "-e", "console.log(JSON.stringify(process.cwd()))", cwd=unusual)), str(unusual))
        equal((unusual / "oyzu.lock").read_bytes(), original_locks[0])
        return "spaces and Unicode in project directory; exact cwd and unchanged lock"
    record("project-path-edge-cases", unusual_project_path)
    def linked_project_path():
        linked = run_root / "linked-project"
        kind = "symlink"
        try:
            linked.symlink_to(a, target_is_directory=True)
        except OSError:
            if os.name != "nt":
                raise
            subprocess.run(["cmd", "/c", "mklink", "/J", str(linked), str(a)],
                           check=True, capture_output=True, text=True)
            kind = "junction"
        equal(call("exec", "--", "node", "--version", cwd=linked), "v" + VERSIONS[0])
        equal((linked / "oyzu.lock").read_bytes(), original_locks[0])
        return {"kind": kind, "selection": "locked Node", "lock_unchanged": True}
    record("linked-project-path", linked_project_path)

    def termination():
        heartbeat = run_root / "termination-heartbeat"
        script = ("const fs=require('fs');const p=" + json.dumps(str(heartbeat)) + ";"
                  "fs.writeFileSync(p,'ready');console.log(process.pid);"
                  "setInterval(()=>fs.writeFileSync(p,String(Date.now())),50)")
        process = subprocess.Popen([str(binary), "exec", "--", "node", "-e", script], cwd=a,
                                   env=base_env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   text=True, encoding="utf-8")
        tool_pid = None
        try:
            ready = queue.Queue()
            threading.Thread(target=lambda: ready.put(process.stdout.readline()), daemon=True).start()
            tool_pid = int(ready.get(timeout=15).strip())
            process.terminate()
            process.wait(timeout=15)
            time.sleep(.3)
            before = heartbeat.read_bytes()
            time.sleep(.3)
            assert heartbeat.read_bytes() == before, "tool survived frontend termination (cleaned up by harness)"
            return {"frontend_pid_equals_tool_pid": process.pid == tool_pid, "heartbeat_stopped": True}
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=10)
            if tool_pid is not None and tool_pid != process.pid:
                try:
                    os.kill(tool_pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            process.stdout.close()
            process.stderr.close()
    record("frontend-termination-stops-tool", termination)
    if os.name != "nt":
        record("non-utf8-environment-hook", lambda: call("hook-env", "-s", "bash", cwd=a,
                                                       env=dict(base_env, OYZU_INVALID=os.fsdecode(b"\xff"))) and "hook emitted successfully")

    def ambient():
        (a / "mise.toml").write_text('[tools]\nnode="99"\n[env]\nOYZU_PROJECT="ambient"\n')
        (a / "mise.lock").write_text('deliberately invalid mise lock')
        (a / ".tool-versions").write_text('node 99\n')
        hostile = dict(base_env, MISE_NODE_VERSION="99", MISE_BACKENDS_NODE="http:unapproved", MISE_CONFIG_FILE=str(a / "mise.toml"))
        return equal(call("exec", "--", "node", "-p", "process.version + ':' + process.env.OYZU_PROJECT", cwd=a, env=hostile), "v22.14.0:22")

    record("ambient-mise-cannot-override-oyzu", ambient)
    nested = a / "nested"
    nested.mkdir()
    record("nested-directory-selection", lambda: equal(call("exec", "--", "node", "--version", cwd=nested), "v22.14.0"))
    nested_project = nested / "child-project"
    nested_project.mkdir()
    for filename in ("oyzu.toml", "oyzu.lock"):
        shutil.copyfile(b / filename, nested_project / filename)
    record("nested-project-overrides-parent", lambda: equal(call("exec", "--", "node", "--version", cwd=nested_project), "v24.0.0"))
    def shell_lifecycle(shell_name):
        harness = Path(__file__).resolve().parent
        template = harness / ("shell-test.ps1" if shell_name == "pwsh" else "shell-test.sh")
        script = run_root / (shell_name + template.suffix)
        script.write_text(template.read_text(), newline="\n")
        shell_env = dict(base_env, OYZU_SPIKE_BINARY=str(binary), OYZU_TEST_A=str(a),
                         OYZU_TEST_B=str(b), OYZU_TEST_OUTSIDE=str(run_root),
                         OYZU_TEST_EXTRA=str(run_root / "user-path"), OYZU_TEST_SHELL=shell_name,
                         OYZU_EXPECT_A="v22.14.0", OYZU_EXPECT_B="v24.0.0", OYZU_EXPECT_ENV_A="22", OYZU_EXPECT_ENV_B="24")
        reverse_env = dict(shell_env, OYZU_TEST_A=str(b), OYZU_TEST_B=str(a),
                           OYZU_EXPECT_A="v24.0.0", OYZU_EXPECT_B="v22.14.0", OYZU_EXPECT_ENV_A="24", OYZU_EXPECT_ENV_B="22")
        (run_root / "user-path").mkdir(exist_ok=True)
        invocation = [shell_name, "-NoProfile", "-File", str(script)] if shell_name == "pwsh" else [shell_name, str(script)]
        # Two simultaneous shells prove activation is not a shared global selection.
        with ThreadPoolExecutor(2) as pool:
            jobs = [pool.submit(subprocess.run, invocation, cwd=run_root, env=environment, text=True, capture_output=True, timeout=180) for environment in (shell_env, reverse_env)]
            outputs = [job.result() for job in jobs]
        for output in outputs:
            assert output.returncode == 0, output.stdout + output.stderr
            assert "shell lifecycle passed" in output.stdout, output.stdout
        return "two independent shells: entry, switch, exit, PATH and user edits"
    for shell_name in (["pwsh"] if os.name == "nt" else ["bash", "zsh"]):
        if shutil.which(shell_name):
            record(f"shell-{shell_name}", lambda shell_name=shell_name: shell_lifecycle(shell_name))
    record("cached-policy-denial", lambda: denied("policy denies locked version", "exec", "--", "node", "--version", cwd=a, env=dict(base_env, OYZU_SPIKE_DENY_VERSION=VERSIONS[0])))
    for mode in ("corrupt", "redirect", "missing"):
        expected = {"corrupt": "checksum mismatch", "redirect": "route denied", "missing": "precompiled node archive not found"}[mode]
        record(f"acquisition-{mode}-denied", lambda mode=mode, expected=expected: denied(expected, "install", cwd=a, env=dict(base_env, OYZU_SPIKE_STATE=str(run_root/mode), OYZU_SPIKE_ROUTE=route.replace("/ok", "/"+mode))))
    record("unavailable-proxy-denied", lambda: call("install", cwd=a, env=dict(base_env, OYZU_SPIKE_STATE=str(run_root/"unavailable"), OYZU_SPIKE_ROUTE="http://127.0.0.1:1/ok"), code=1))
    record("redirect-target-never-contacted", lambda: equal(any(p.startswith("/forbidden") for p in requests), False))

    def concurrent():
        fresh = dict(base_env, OYZU_SPIKE_STATE=str(run_root / "concurrent"))
        first_request = len(requests)
        with ThreadPoolExecutor(2) as pool:
            values = list(pool.map(lambda _: call("install", cwd=a, env=fresh), range(2)))
        equal(call("exec", "--", "node", "--version", cwd=a, env=fresh), "v22.14.0")
        archive_requests = [p for p in requests[first_request:] if p.endswith(inventory[VERSIONS[0]]["name"])]
        equal(len(archive_requests), 1)
        return [json.loads(v)["versions"] for v in values]

    record("concurrent-install", concurrent)
    def active_version():
        script = "console.log(process.version);process.stdin.once('data',()=>{console.log(process.version);process.exit(0)})"
        child = subprocess.Popen([str(binary), "exec", "--", "node", "-e", script], cwd=a,
                                 env=base_env, text=True, stdin=subprocess.PIPE,
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            ready = queue.Queue()
            threading.Thread(target=lambda: ready.put(child.stdout.readline()), daemon=True).start()
            equal(ready.get(timeout=15).strip(), "v22.14.0")
            equal(call("exec", "--", "node", "--version", cwd=b), "v24.0.0")
            output, error = child.communicate("resume\n", timeout=15)
            equal(child.returncode, 0)
            equal(output.strip(), "v22.14.0")
            return "Node 22 stayed active while another invocation selected Node 24"
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=10)
    record("active-version-independence", active_version)
    record("reshim", lambda: call("reshim", cwd=a))
    shim_name = ("node.exe" if args.windows_shim_mode == "hardlink" else "node.cmd") if os.name == "nt" else "node"
    shim = Path(base_env["OYZU_SPIKE_STATE"]) / "data" / "shims" / shim_name

    def shim_run():
        output = subprocess.check_output([str(shim), "--version"], cwd=b, env=base_env, text=True, timeout=60).strip()
        return equal(output, "v24.0.0")

    record("shim-without-activation", shim_run)

    def shim_arguments():
        expected = ["a b", "single'quote", 'double"quote', "$literal"]
        output = subprocess.check_output([str(shim), "-e", "console.log(JSON.stringify(process.argv.slice(1)))", "--", *expected],
                                         cwd=b, env=base_env, text=True, timeout=60)
        return equal(json.loads(output), expected)

    record("shim-arguments", shim_arguments)
    def shim_argument_edges():
        output = subprocess.check_output([str(shim), "-e", "console.log(JSON.stringify(process.argv.slice(1)))", "--", *edge_arguments],
                                         cwd=b, env=base_env, text=True, encoding="utf-8", timeout=60)
        return equal(json.loads(output), edge_arguments)
    record("shim-argument-edge-cases", shim_argument_edges)
    record("shim-exit-status", lambda: equal(subprocess.run([str(shim), "-e", "process.exit(37)"],
                                                           cwd=b, env=base_env, capture_output=True, timeout=60).returncode, 37))

    def frozen():
        for p, expected in zip(projects, original_locks):
            equal((p / "oyzu.lock").read_bytes(), expected)
        equal((a / "mise.lock").read_text(), "deliberately invalid mise lock")
        equal(list(b.glob("mise.*")), [])
        return "Oyzu locks unchanged; no generated mise project files"

    record("frozen-lock-and-file-ownership", frozen)
    if args.network_isolated:
        def subprocess_egress():
            output = subprocess.run(["curl", "--noproxy", "*", "--connect-timeout", "3", "http://1.1.1.1"], capture_output=True, timeout=10)
            assert output.returncode != 0, "network isolation ineffective"
            return {"exit": output.returncode, "stderr": output.stderr.decode().strip()}
        record("subprocess-egress-denied-by-container", subprocess_egress)

    def benchmark():
        # Preserve the fixture cache, then measure one fresh process with an
        # empty mise cache. This does not evict the operating system's page cache.
        cache = Path(base_env["OYZU_SPIKE_STATE"]) / "cache"
        saved_cache = cache.with_name("cache-before-benchmark")
        assert cache.resolve().is_relative_to(run_root.resolve())
        assert saved_cache.resolve().is_relative_to(run_root.resolve())
        if cache.exists():
            cache.rename(saved_cache)
        cold_start = time.perf_counter()
        call("hook-env", "-s", "pwsh" if os.name == "nt" else "bash", cwd=a)
        cold_ms = (time.perf_counter() - cold_start) * 1000
        samples = []
        for _ in range(21):
            start = time.perf_counter()
            call("hook-env", "-s", "pwsh" if os.name == "nt" else "bash", cwd=a)
            samples.append((time.perf_counter()-start)*1000)
        return {"cold_mise_cache_ms": cold_ms, "first_ms": samples[0], "repeated_p50_ms": statistics.median(samples[1:]), "repeated_p95_ms": sorted(samples[1:])[18], "scope": "empty mise cache then repeated hooks; fresh process per hook; OS filesystem caches not evicted; not an upstream performance comparison"}
    record("hook-timings", benchmark)
    record("paired-in-process-benchmark", lambda: json.loads(call("benchmark", cwd=a)))
    server.shutdown()
    print(f"Evidence: {run_root / 'results.json'}", flush=True)
    if any(r["status"] != "passed" for r in results):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
