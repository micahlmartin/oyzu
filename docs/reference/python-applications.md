# Python application artifacts

An explicit `python/app` target with static PEP 621 metadata and one console script produces a runnable, versioned `.pyz` application alongside the native wheel and source distribution. The application contains the built wheel's files and captured runtime dependencies. It does not copy arbitrary checkout files or rebuild the project for packaging.

```yaml
api:
  uses: python/app
```

The native `pyproject.toml` supplies application intent:

```toml
[project]
name = "example-api"
version = "0.1.0"
dependencies = []

[project.scripts]
example-api = "api:main"

[build-system]
requires = ["setuptools==80.9.0"]
build-backend = "setuptools.build_meta"

[tool.setuptools]
packages = ["api"]
```

`api.main` is an ordinary console callable. Oyzu does not invent a server, port, health check or supervisor. Multiple scripts or missing scripts fail planning with a focused diagnostic; an entrypoint-selection override remains pending. Auto-detected `python/package` projects retain package semantics and may contain multiple console scripts.

## Build and use

```text
oyzu run list
oyzu build
oyzu inspect dist
python <application-artifact-path-from-manifest>
```

Read the `application` artifact's path from `dist/manifest.json`. Its filename is `<target>-<snapshot-version>.pyz`, using the package's PEP 440 snapshot version, such as `0.1.0.dev0+g0123456789ab`. `wheel` and `sdist` remain separate artifacts. Existing consumers must select artifacts by identity rather than assuming an application emits exactly two files.

The pipeline prepares captured dependencies, invokes the selected native backend once, assembles the application, tests sources extracted from that archive, and runs lint/read-only formatting gates. Shared collection retains JUnit and Cobertura in `dist/` with paths, digests and summaries in the manifest. See [Python testing](python-testing.md) for collection, overrides, empty-suite behavior and coverage thresholds. Direct development tasks retain native commands; they do not assemble application archives or collect build evidence automatically.

Assembly checks the native wheel's identity and selected console entrypoint against the plan. Native pip installs only captured runtime wheels into the payload offline, without dependency resolution; build/test tools remain outside the payload. Installer-local provenance and interpreter-specific launchers are removed. A generated launcher uses native distribution entrypoint metadata. The internal `oyzu-application.json` records source identity, version, project source files, runtime package identities, selected entrypoint and original wheel digest.

Archive entries have sorted paths and normalized ZIP metadata. Shared archive validation rejects unsafe paths, case collisions, symlinks and oversized payloads. Packaging verifies the archive's tested digest and unchanged native wheel/sdist inputs. Tests, missing reports, quality failures and changed artifacts block successful collection. Inspect retained logs/reports in `dist/` to correct the failing step. These checks bind the normal lifecycle; they do not confer production trust or protect against malicious tests rewriting private build state.

## Prerequisites, compatibility and limits

Captured builds use provisioned Linux container images through Docker, as described in the [CLI overview](README.md), with Python 3.12 and pip, uv or Poetry backend integration. The CLI does not install tools. Preparation uses existing broker routes; assembly and tests use captured inputs with networking disabled. Full private registry support remains pending. Correct missing inputs during preparation rather than adding a network fallback to the build.

The archive requires pure Python wheels and zip-compatible code/data. Native extensions, `.pth` startup behavior and libraries needing real filesystem resources are not generally ZIP-compatible. Native-wheel and forbidden-payload checks fail explicitly; native tests must establish the application's resource-loading behavior. Python itself is not bundled: running the artifact requires a compatible interpreter. Legacy setup.py applications, dynamic console metadata, editable installs, general entrypoint overrides and native application layouts remain unfinished. Package builders retain native-extension support where implemented.

Requirements-only applications continue using conventional `app.py` or `__main__.py` without invented distribution metadata and retain their existing `0.0.0-dev.g<source>` snapshot versions. Both application paths share archive writing, archive-source tests and packaging checks. No Oyzu test/output configuration is required.

Optional [Dockerfile-free Python container assembly](python-containers.md) now consumes this exact tested archive through a captured CPython 3.12 profile. Its new native acceptance case remains pending CI. Runtime overrides, application-container matrices, image startup smoke tests and full profile/policy qualification remain unfinished.

## Verification

```text
python tooling/test-python-distribution-app.py --wheels <prepared-native-wheel-directory>
python tooling/test-python-application.py --wheels <prepared-native-wheel-directory>
```

Supply wheels for the host interpreter: packaging 24.2, build 1.2.2.post1, setuptools 80.9.0, wheel 0.45.1, pytest 8.3.5, pytest-cov 6.0.0, Ruff 0.11.13 and dependencies. The probes do not download them. The distribution probe builds a real setuptools wheel, executes the archive without site packages, replaces checkout code before testing, checks runtime-only inclusion and repeated artifact identity, and rejects entrypoint mismatch and post-test changes. Windows native verification does not establish Linux isolation. CI separately exercises EX-012 through the compiled CLI, quality failures, root/configured suites, reports and repeatability. Consult [implementation status](../implementation-status.md) for revision-specific results.
