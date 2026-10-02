"""Build the qualification worker from an explicitly supplied Linux frontend."""
import argparse
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--context", type=Path, required=True)
    parser.add_argument("--image", default="oyzu-mise-broker-qualification:local")
    args = parser.parse_args()
    with args.binary.open("rb") as binary:
        assert binary.read(4) == b"\x7fELF", "the worker requires a Linux binary"
    here = Path(__file__).resolve().parent
    root = here.parents[1]
    context = args.context.resolve()
    assert not context.is_relative_to(root), "keep generated image context outside the repository"
    context.mkdir(parents=True, exist_ok=True)
    for name in ("broker_bridge.py", "broker_worker.py", "UPSTREAM-LICENSE.txt", "dependency-licenses.json"):
        shutil.copyfile(here / name, context / name)
    shutil.copyfile(here / "Broker.Dockerfile", context / "Dockerfile")
    shutil.copyfile(root / "src/broker/runtime/transport.py", context / "broker_transport.py")
    shutil.copyfile(args.binary, context / "oyzu-mise-spike")
    subprocess.run(["docker", "build", "-t", args.image, str(context)], check=True)


if __name__ == "__main__":
    main()
