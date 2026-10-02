#!/usr/bin/env python3
"""Record actual Linux exec/file/network activity for an installed fixture run."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--run-root", required=True, type=Path)
    args = parser.parse_args()
    root = args.run_root.resolve()
    binary = args.binary.resolve()
    environment = dict(os.environ, OYZU_SPIKE_STATE=str(root / "state"),
                       OYZU_SPIKE_ROUTE="http://127.0.0.1:1/closed")
    traces = root / "traces"
    traces.mkdir(exist_ok=True)
    evidence = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "cases": []}
    for name, command in (("hook", ["hook-env", "-s", "bash"]),
                          ("exec", ["exec", "--", "node", "--version"])):
        log = traces / (name + ".log")
        completed = subprocess.run(["strace", "-f", "-e", "trace=%file,%process,%network",
                                    "-o", str(log), str(binary), *command],
                                   cwd=root / "project-22.14.0", env=environment,
                                   capture_output=True, text=True, timeout=30)
        assert completed.returncode == 0, completed.stderr
        text = log.read_text()
        counts = Counter(re.findall(r"^\d+\s+(\w+)\(", text, flags=re.MULTILINE))
        executions = re.findall(r'execve\("([^"]+)"', text)
        connections = [line for line in text.splitlines() if "connect(" in line]
        assert all(Path(p).name not in ("mise", "mise.exe") for p in executions), executions
        assert not any("AF_INET" in line for line in connections), connections
        assert len(executions) == (1 if name == "hook" else 2), executions
        evidence["cases"].append({"case": name, "executions": executions,
                                   "syscall_counts": dict(sorted(counts.items())),
                                   "connections": connections,
                                   "stdout": completed.stdout if name == "exec" else "shell delta emitted",
                                   "trace_sha256": hashlib.sha256(log.read_bytes()).hexdigest()})
    (traces / "summary.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
