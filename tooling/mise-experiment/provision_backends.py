"""Provision real, independently verified backend fixtures outside the repo."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work", type=Path, required=True)
    args = parser.parse_args()
    destination = args.work.resolve() / "backend-archives"
    repository = Path(__file__).resolve().parents[2]
    assert not destination.is_relative_to(repository)
    destination.mkdir(parents=True, exist_ok=True)
    metadata_url = "https://go.dev/dl/?mode=json&include=all"
    with urllib.request.urlopen(metadata_url, timeout=60) as response:
        metadata = response.read()
    (destination / "go-downloads.json").write_bytes(metadata)
    release = next(r for r in json.loads(metadata) if r["version"] == "go1.24.1")
    item = next(f for f in release["files"] if f["os"] == "linux" and
                f["arch"] == "amd64" and f["kind"] == "archive")
    archive = destination / item["filename"]
    url = "https://dl.google.com/go/" + item["filename"]
    if not archive.exists():
        temporary = archive.with_suffix(archive.suffix + ".partial")
        with urllib.request.urlopen(url, timeout=120) as response, temporary.open("wb") as output:
            while chunk := response.read(1024 * 1024):
                output.write(chunk)
        temporary.replace(archive)
    assert archive.stat().st_size == item["size"]
    with archive.open("rb") as content:
        assert hashlib.file_digest(content, "sha256").hexdigest() == item["sha256"]
    inventory = {"go": dict(item, version="1.24.1", url=url,
                           metadata_url=metadata_url,
                           metadata_sha256=hashlib.sha256(metadata).hexdigest())}
    java_metadata_url = "https://mise-java.jdx.dev/jvm/ga/linux/x86_64.json"
    java_metadata_path = destination / "java-linux-x64.json"
    subprocess.run(["curl", "--fail", "--silent", "--show-error", "--location",
                    java_metadata_url, "--output", str(java_metadata_path)], check=True)
    java_metadata = java_metadata_path.read_bytes()
    java = next(m for m in json.loads(java_metadata) if m["vendor"] == "temurin" and
                m["version"] == "8.0.442+6" and m["image_type"] == "jdk" and m["features"] == [])
    java_archive = destination / java["url"].rsplit("/", 1)[1]
    if not java_archive.exists():
        temporary = java_archive.with_suffix(".partial")
        subprocess.run(["curl", "--fail", "--silent", "--show-error", "--location",
                        java["url"], "--output", str(temporary)], check=True)
        temporary.replace(java_archive)
    with java_archive.open("rb") as content:
        java_digest = hashlib.file_digest(content, "sha256").hexdigest()
    assert "sha256:" + java_digest == java["checksum"]
    inventory["java"] = dict(java, filename=java_archive.name, sha256=java_digest,
                             size=java_archive.stat().st_size, locked_version="temurin-8.0.442+6",
                             metadata_url=java_metadata_url,
                             metadata_sha256=hashlib.sha256(java_metadata).hexdigest())
    python_name = "cpython-3.12.9+20250317-x86_64-unknown-linux-gnu-install_only.tar.gz"
    python_url = "https://github.com/astral-sh/python-build-standalone/releases/download/20250317/" + python_name
    python_archive = destination / python_name
    checksum = subprocess.check_output(["curl", "--fail", "--silent", "--show-error", "--location",
                                        python_url + ".sha256"], text=True).strip()
    assert len(checksum) == 64 and all(c in "0123456789abcdef" for c in checksum)
    if not python_archive.exists():
        temporary = python_archive.with_suffix(".partial")
        subprocess.run(["curl", "--fail", "--silent", "--show-error", "--location",
                        python_url, "--output", str(temporary)], check=True)
        temporary.replace(python_archive)
    with python_archive.open("rb") as content:
        assert hashlib.file_digest(content, "sha256").hexdigest() == checksum
    attestation_url = "https://api.github.com/repos/astral-sh/python-build-standalone/attestations/sha256:" + checksum
    attestations = destination / "python-attestations.json"
    subprocess.run(["curl", "--fail", "--silent", "--show-error", "--location", attestation_url,
                    "--output", str(attestations)], check=True)
    assert json.loads(attestations.read_bytes())["attestations"]
    inventory["python"] = dict(filename=python_name, version="3.12.9", url=python_url,
                               sha256=checksum, size=python_archive.stat().st_size,
                               attestation_url=attestation_url,
                               attestation_sha256=hashlib.sha256(attestations.read_bytes()).hexdigest())
    jq_url = "https://github.com/jqlang/jq/releases/download/jq-1.7.1/jq-linux-amd64"
    jq_checksums = subprocess.check_output(["curl", "--fail", "--silent", "--show-error", "--location",
                                           jq_url.rsplit("/", 1)[0] + "/sha256sum.txt"])
    jq_digest = next(line.split()[0] for line in jq_checksums.decode().splitlines()
                     if line.split()[-1] == "jq-linux-amd64")
    jq_archive = destination / "jq-linux-amd64"
    if not jq_archive.exists():
        subprocess.run(["curl", "--fail", "--silent", "--show-error", "--location",
                        jq_url, "--output", str(jq_archive)], check=True)
    with jq_archive.open("rb") as content:
        assert hashlib.file_digest(content, "sha256").hexdigest() == jq_digest
    (destination / "jq-sha256sum.txt").write_bytes(jq_checksums)
    jq_release = subprocess.check_output(["curl", "--fail", "--silent", "--show-error", "--location",
                                          "https://api.github.com/repos/jqlang/jq/releases/tags/jq-1.7.1"])
    (destination / "jq-release.json").write_bytes(jq_release)
    inventory["jq"] = dict(filename=jq_archive.name, version="1.7.1", url=jq_url,
                           sha256=jq_digest, size=jq_archive.stat().st_size,
                           release_sha256=hashlib.sha256(jq_release).hexdigest(),
                           checksums_sha256=hashlib.sha256(jq_checksums).hexdigest())
    (destination / "inventory-linux-x64.json").write_text(json.dumps(inventory, indent=2))
    print(json.dumps(inventory, indent=2))


if __name__ == "__main__":
    main()
