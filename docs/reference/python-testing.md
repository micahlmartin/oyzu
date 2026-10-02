# Python test discovery and reports

Python builders expose an implicit `test` task for pip, uv and Poetry projects. It remains callable and part of the default build even without a `tests/` or `test/` directory. Native pytest selects tests at execution time, including root-level tests and configured paths. Oyzu does not import `conftest.py`, execute Python or collect tests during static discovery.

## Selection and commands

```text
oyzu run list
oyzu run test
oyzu build
oyzu inspect dist
```

The static pytest detector records evidence from `pytest.ini`, `.pytest.ini`, `[tool.pytest.ini_options]` in `pyproject.toml`, `[pytest]` in `tox.ini`, or `[tool:pytest]` in `setup.cfg`. With no explicit evidence, it records pytest as the profile default. This identifies intent; it does not parse native selection rules or decide which tests exist. Native pytest owns configuration precedence, `testpaths`, filename patterns, markers and collection errors. Multiple pytest configuration files remain inputs for the same framework, not competing frameworks.

For example, this native configuration requires no Oyzu test section:

```ini
[pytest]
testpaths = checks
python_files = spec_*.py
```

Development execution uses the detected manager:

| Manager | Implicit command |
| --- | --- |
| pip | `python -m pytest` |
| uv | `uv run --locked pytest` |
| Poetry | `poetry run pytest` |

Provision Python, the manager and project dependencies before using these commands. Development execution retains the native manager's environment and behavior, including any native synchronization; it is not an isolated build. Explicit TOML task replacements and Oyzu pre/post hooks retain their normal task-engine behavior. Additional arguments can be forwarded with `oyzu run test -- <pytest arguments>`.

## Captured builds and evidence

`oyzu build` requires the provisioned container toolchain and dependency preparation described in the [CLI overview](README.md). Preparation includes pytest 8.3.5 and pytest-cov 6.0.0 when the project/native lock has not selected those packages. Reporter preparation no longer depends on a guessed test directory. Project declarations and native lock constraints keep their versions; incompatible integrations fail rather than downloading a replacement during tests. Current Python acquisition uses the public PyPI broker routes; complete private-source integration remains pending.

The package test adapter runs pytest against the installed snapshot distribution with importlib mode. It derives measurable Python sources from installed wheel metadata and preserves native coverage settings, including thresholds. The application adapter measures source extracted from its generated application artifact. Native pytest/pytest-cov create JUnit and Cobertura; shared collection retains them under the target's reports in `dist/` and records paths, digests and parsed summaries in `dist/manifest.json`. No test/output YAML is needed. Successful test and quality gates permit the builder's snapshot artifacts to be collected.

An assertion failure, collection error or failed coverage threshold blocks final artifact collection. An empty suite keeps pytest's native exit code 5 and any emitted zero-test JUnit report; it is not rewritten as successful test evidence. Coverage that was not produced remains missing, while emitted coverage retains its actual denominator and hits. A suite containing explicitly skipped tests retains native skip counts and the native exit outcome. A zero exit code from a custom replacement cannot satisfy missing required reports by itself.

Direct `oyzu run test` currently preserves native console output and exit status but does not automatically create a dist/report bundle. Use `oyzu build` for collected evidence. No-tests behavior is a compatibility change: projects previously excluded from the test stage by the directory heuristic now invoke pytest and can fail the build when it collects no tests. Add tests or correct native collection configuration; an empty directory is not a successful test result.

## Verification, limits and recovery

```text
python tooling/test-python-tasks.py --cli target/debug/oyzu
python tooling/test-python-reporting.py
python tooling/test-python-adapter.py
```

Use `target/debug/oyzu.exe` on Windows. The probes need installed pytest/pytest-cov; acquisition-boundary checks also need pip. Local verification uses Python 3.12.14, pytest 8.3.5, pytest-cov 6.0.0 and coverage 7.16.2 on Windows. CI runs direct task/reporting checks on Windows, macOS and Linux, then separate captured-build cases on Linux. See [implementation status](../implementation-status.md) for revision-specific results; configured CI is not evidence that a pending job passed.

The native probes verify root and configured suites, nonexecuting listing, assertion failures/skips, empty-suite exit/JUnit, installed-source coverage and native coverage threshold failure. Captured scenarios additionally require snapshot wheel/sdist artifacts, report collection, unchanged checkout bytes and rejection of artifacts after an empty suite. Native reporter tests alone do not verify container isolation or packaging.

For no tests, check pytest's native filename patterns and configuration, then run the same manager command to inspect collection. For missing reporters, correct the declared dependencies or provisioned environment and repeat preparation. For report/coverage failure, inspect the native logs and report records retained in `dist/`. Native test configuration and plugins execute during testing, so they are subject to the build's offline execution boundary, or ordinary host permissions for direct tasks.

This integration uses pytest as the supported default. Pytest can run supported `unittest.TestCase` suites, but a dedicated unittest runner preserving every unittest feature is not implemented. Other frameworks, general custom-runner adaptation, full configuration inheritance, all pytest versions and standalone task evidence remain separate work. Missing measurable Python source is still an explicit adapter error; native-extension coverage is not claimed from Python line measurements.

Native references: [pytest configuration](https://docs.pytest.org/en/stable/reference/customize.html) and [pytest exit codes](https://doc.pytest.org/en/8.2.x/reference/exit-codes.html).
