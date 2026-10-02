"""Measure a real asdf plugin's independent native-client download path."""
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import urllib.request

from broker_bridge import start


def main():
    config = json.loads(Path("/workspace/worker.json").read_text())
    server, bridge = start(config["routes"])
    plugin = config["plugin"]
    with urllib.request.urlopen(bridge + "/asdf/plugin.tar.gz", timeout=60) as response:
        archive = response.read()
    assert hashlib.sha256(archive).hexdigest() == plugin["sha256"]
    state = Path("/tmp/mise-asdf")
    plugins = state / "data/plugins"
    plugins.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(archive)) as package:
        # The pinned image's Python predates tarfile's data filter. Accept only
        # ordinary files/directories confined to this fresh private directory.
        for member in package.getmembers():
            name = PurePosixPath(member.name)
            assert not name.is_absolute() and ".." not in name.parts
            assert member.isfile() or member.isdir(), "plugin archive contains a link or special file"
        package.extractall(plugins)
    # Upstream canonicalizes the public "golang" spelling to its "go" directory.
    (plugins / ("asdf-golang-" + plugin["pin"])).rename(plugins / "go")
    assert hashlib.sha256((plugins / "go/LICENSE").read_bytes()).hexdigest() == plugin["license_sha256"]
    project = Path("/workspace/asdf-project")
    project.mkdir()
    (project / "oyzu.toml").write_text('[tools]\ngolang = "1.24.1"\n')
    go = config["go"]
    lock = f'''format = 1
[[tool]]
id = "asdf:golang"
request = "1.24.1"
version = "1.24.1"
backend_digest = "git:da0db43e9398b46bafa95232014708a51120e731"
[[tool.distribution]]
platform = "linux-x64"
digest = "sha256:{go['sha256']}"
size = {go['size']}
source_id = "go-fixture"
verification = "fixture-sha256"
dependencies = []
'''
    (project / "oyzu.lock").write_text(lock)
    artifacts = project / "artifact-routes.json"
    artifacts.write_text(json.dumps({"go-fixture:sha256:" + go["sha256"]: bridge + "/go/" + go["filename"]}))
    environment = dict(os.environ, HOME="/tmp/home", OYZU_SPIKE_STATE=str(state),
                       OYZU_SPIKE_ROUTE=bridge + "/go", OYZU_SPIKE_ARTIFACTS=str(artifacts),
                       OYZU_SPIKE_BACKENDS=json.dumps({"golang": "asdf:golang"}),
                       OYZU_SPIKE_URL_REPLACEMENTS=json.dumps({"https://dl.google.com/go": bridge + "/go"}))
    result = subprocess.run(["/usr/local/bin/oyzu-mise-spike", "install"], cwd=project,
                            env=environment, capture_output=True, text=True, timeout=90)
    Path("/out/install-stdout.txt").write_text(result.stdout)
    Path("/out/install-stderr.txt").write_text(result.stderr)
    Path("/out/bridge-requests.json").write_text(json.dumps(server.responses, indent=2))
    assert result.returncode != 0, "unmediated plugin unexpectedly installed"
    assert (project / "oyzu.lock").read_text() == lock
    assert not any(request.startswith("/go/") for request in server.requests), "plugin unexpectedly used the mise HTTP route"
    assert "bin/download" in result.stderr and "exit code 6" in result.stderr, result.stderr
    execution = subprocess.run(["/usr/local/bin/oyzu-mise-spike", "exec", "--", "go", "version"],
                               cwd=project, env=environment, capture_output=True, text=True, timeout=30)
    assert execution.returncode != 0 and "not installed" in execution.stderr, execution.stderr
    results = [{"case": "real-asdf-plugin-unmediated-download-denied", "status": "passed",
                "detail": {"plugin_pin": plugin["pin"], "exit": result.returncode,
                           "diagnostic": result.stderr.strip(), "scope": "negative containment case; installation unsupported through HTTP replacement alone"}}]
    Path("/out/worker-results.json").write_text(json.dumps(results, indent=2))
    server.shutdown()


if __name__ == "__main__":
    main()
