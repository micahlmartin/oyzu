#!/usr/bin/env bash
# Linux acceptance-tool provisioning and native conformance, not product acquisition.
set -euo pipefail
cd "$(dirname "$0")/.."
mode="${1:-}"
suite="${2:-all}"
case "$suite" in all|core|node|python|go|rust|java|helm|docker) ;; *) echo "Unknown suite: $suite" >&2; exit 2 ;; esac
case "$mode" in provision|native) ;; *) echo "Usage: bash tooling/build-scenario-tools.sh {provision|native} [suite]" >&2; exit 2 ;; esac
selected() {
  if [[ "$suite" == all ]]; then return 0; fi
  for candidate in "$@"; do
    if [[ "$suite" == "$candidate" ]]; then return 0; fi
  done
  return 1
}

if [[ "$mode" == provision ]]; then
  python3 -m venv .ci-python
  .ci-python/bin/python -m pip install -r tooling/design-requirements.txt
  if selected node core; then
    docker pull node:22-bookworm-slim
    docker build --tag oyzu-toolchain/node:quality tooling/images/node-quality
    npm ci --ignore-scripts --no-audit --no-fund --prefix tooling/images/node-quality
    docker build --tag oyzu-toolchain/node:npm11.11.0-node22 tooling/images/node-npm
  fi
  if selected node; then
    docker build --tag oyzu-toolchain/node:pnpm10.11.0-node22 tooling/images/node-pnpm
    docker build --tag oyzu-toolchain/node:yarn1.22.22-node22 tooling/images/node-yarn
  fi
  if selected go docker core; then
    docker pull golang:1.24-bookworm
    docker build --tag oyzu-toolchain/go:1.24-mod0.25.0 --file tooling/images/go.Dockerfile .
  fi
  if selected python core; then
    docker pull python:3.12-slim-bookworm
    docker pull python:3.12-bookworm
    docker pull ghcr.io/astral-sh/uv:0.12.21-python3.12-trixie-slim
    docker build --tag oyzu-toolchain/poetry:2.5.1-python3.12 --file tooling/images/python-poetry.Dockerfile tooling/images
  fi
  if selected python; then
    docker run --rm --mount type=bind,source="$PWD",target=/repository --workdir /repository python:3.12-bookworm python -m pip download --only-binary=:all: --dest .ci-python-legacy-wheels setuptools==80.9.0 wheel==0.45.1 build==1.2.2.post1 packaging==24.2 pytest==8.3.5 pytest-cov==6.0.0 ruff==0.11.13
    .ci-python/bin/python -m pip install pytest==8.3.5 pytest-cov==6.0.0 coverage==7.16.2
    .ci-python/bin/python -m pip download --only-binary=:all: --dest .ci-python-app-wheels packaging==24.2 pytest==8.3.5 pytest-cov==6.0.0 coverage==7.16.2 ruff==0.11.13 build==1.2.2.post1 setuptools==80.9.0 wheel==0.45.1
    .ci-python/bin/python -m pip download --only-binary=:all: --dest .ci-python-quality-wheels black==25.1.0 flake8==7.3.0 ruff==0.11.13
  fi
  if selected rust; then
    docker build --tag oyzu-toolchain/rust:1.94.0-nextest0.9.146-llvmcov0.9.1 --file tooling/images/rust.Dockerfile tooling/images
  fi
  if selected helm; then
    docker build --tag oyzu-toolchain/helm:3.22.0 --file tooling/images/helm.Dockerfile tooling/images
  fi
  if selected java; then
    docker build --tag oyzu-toolchain/ant:1.10.18-jdk17 --file tooling/images/ant.Dockerfile .
    docker build --tag oyzu-toolchain/maven:3.9.11-jdk17 --file tooling/images/maven.Dockerfile .
    docker build --tag oyzu-toolchain/gradle:8.14.3-jdk17 --file tooling/images/gradle.Dockerfile .
  fi
  if selected docker go; then
    docker pull alpine:3.22
    docker tag alpine:3.22 oyzu-fixture/alpine:amd64
    docker build --tag oyzu-toolchain/docker:buildkit0.25.0 --file tooling/images/docker-metadata.Dockerfile .
    sudo apparmor_parser -r tooling/images/buildkit.apparmor
  fi
else
  if selected python; then
    .ci-python/bin/python tooling/test-python-adapter.py
    .ci-python/bin/python tooling/test-python-reporting.py
    .ci-python/bin/python tooling/test-python-application.py --wheels .ci-python-app-wheels
    .ci-python/bin/python tooling/test-python-distribution-app.py --wheels .ci-python-app-wheels
    .ci-python/bin/python tooling/test-python-quality.py --wheels .ci-python-quality-wheels
    docker run --rm --network=none --mount type=bind,source="$PWD",target=/repository,readonly --workdir /repository python:3.12-bookworm python tooling/test-python-legacy.py --wheels .ci-python-legacy-wheels
  fi
  if selected helm; then
    .ci-python/bin/python tooling/test-helm-archive.py
    docker run --rm --network=none --mount type=bind,source="$PWD",target=/repository,readonly --workdir /repository --entrypoint python oyzu-toolchain/helm:3.22.0 tooling/test-helm-reporting.py
    HELM_CONTAINER=$(docker create oyzu-toolchain/helm:3.22.0)
    trap 'docker rm -f "$HELM_CONTAINER" >/dev/null' EXIT
    mkdir -p .ci-helm .ci-helm-plugins
    docker cp "$HELM_CONTAINER:/usr/local/bin/helm" .ci-helm/helm
    docker cp "$HELM_CONTAINER:/opt/oyzu-helm-plugins/." .ci-helm-plugins/
    docker rm "$HELM_CONTAINER"
    trap - EXIT
    HELM_PLUGINS="$PWD/.ci-helm-plugins" .ci-python/bin/python tooling/test-helm-reporting.py --helm "$PWD/.ci-helm/helm" --cli cli/oyzu
  fi
  if selected java; then
    .ci-python/bin/python tooling/test-ant-adapter.py
    docker run --rm --network=none --mount type=bind,source="$PWD",target=/repository,readonly --workdir /repository --entrypoint python3 oyzu-toolchain/ant:1.10.18-jdk17 tooling/test-ant-reporting.py --java-home /opt/java/openjdk --ant-home /opt/ant --jacoco-home /opt/jacoco
    docker run --rm --network=none --mount type=bind,source="$PWD",target=/repository,readonly --workdir /repository --entrypoint python3 oyzu-toolchain/maven:3.9.11-jdk17 tooling/test-maven-metadata.py --maven-home /opt/maven --java-home /opt/java/openjdk
    docker run --rm --network=none --mount type=bind,source="$PWD",target=/repository,readonly --workdir /repository --entrypoint python3 oyzu-toolchain/gradle:8.14.3-jdk17 tooling/test-gradle-metadata.py --gradle-home /opt/gradle-8.14.3 --java-home /opt/java/openjdk
  fi
fi
