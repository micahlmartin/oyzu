"""Verify the actual executor does not fall back when its image is unavailable."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = args.work.resolve() / ("executor-denial-" + uuid.uuid4().hex)
    for name in ("workspace", "output", "spool", "private"):
        (root / name).mkdir(parents=True)
    config = {"image": "oyzu-qualification-missing:" + uuid.uuid4().hex,
              "workspace": str(root / "workspace"), "output": str(root / "output"),
              "spool": str(root / "spool"), "private": str(root / "private"), "sources": [],
              "argv": ["sh", "-c", "touch /out/unexpected-execution"], "env": {},
              "timeout_seconds": 10, "name": "oyzu-denial-" + uuid.uuid4().hex[:12]}
    configuration = root / "private/session.json"
    configuration.write_text(json.dumps(config))
    result = subprocess.run([str(args.driver.resolve()), str(configuration)], capture_output=True, text=True, timeout=30)
    assert result.returncode != 0 and "unavailable: pre-provision it before building" in result.stderr, result.stderr
    assert not list((root / "output").iterdir()), "execution or output capture unexpectedly started"
    evidence = {"case": "actual-executor-missing-image-denied", "status": "passed",
                "exit": result.returncode, "diagnostic": result.stderr.strip(), "output_directory_empty": True,
                "driver_sha256": hashlib.sha256(args.driver.read_bytes()).hexdigest(),
                "scope": "real Docker inspection and production executor; no image pull or host execution fallback"}
    args.output.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
