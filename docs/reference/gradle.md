# Gradle builds and tests

Status: experimental. Oyzu detects `build.gradle` or `build.gradle.kts` without a build YAML file. A native multi-project or composite build is one Oyzu target: Gradle owns project membership, included builds, task dependencies and native output paths. Oyzu uses that model to plan artifacts and collect reports. The [EX-024 project](../../examples/builds/java-gradle-multi-project/README.md) demonstrates an application, library and included support build.

## Commands and prerequisites

```text
oyzu run list
oyzu run test
oyzu inspect dist
oyzu build
```

Static task discovery does not execute Gradle. It exposes native `build` and `test` tasks and the shared [Java lint/format tasks](java-quality.md). Native `build` already includes tests, so captured builds do not repeat a separate test stage. Qualified task names and TOML replacements follow the common task engine. No test/report configuration is needed for the default Java test tasks.

Direct tests require provisioned Gradle 8.14.3, JDK 17, Python 3.11+ and native project/test dependencies, including JaCoCo 0.8.13. Set `GRADLE_HOME` or put Gradle on PATH, and configure `JAVA_HOME` for the JDK. The native Gradle user home and project repositories remain authoritative. An existing wrapper is used when discovered; its distribution must already be provisioned. Oyzu does not install tools. Product model/test commands pass `--offline` and disable automatic Java toolchain downloads. Missing cached dependencies fail explicitly; offline dependency resolution does not sandbox arbitrary native build logic or prevent a wrapper from trying to provision itself.

Captured builds additionally require Docker and the provisioned toolchain image:

```text
docker build -f tooling/images/gradle.Dockerfile -t oyzu-toolchain/gradle:8.14.3-jdk17 .
oyzu -C examples/builds/java-gradle-multi-project/project build
```

The CLI does not pull a missing image. Captured acceptance runs on Linux; portable host tasks do not establish isolated executor support on every platform.

## Direct test bundles

After common task admission, the builder evaluates native models for the root and included builds. The observation command requests only Oyzu's metadata tasks; it does not deliberately execute project lifecycle tests or compilation. Gradle configuration, plugins and native build logic still execute normally. This is explicit host execution, not static discovery or a security boundary.

The model identifies enabled test tasks with attributable source files, their native JUnit destinations and injected JaCoCo destinations. Projects without test sources do not create test obligations. Builds with no discovered tests fail rather than manufacturing a passing suite. The exact implicit command is instrumented to run the discovered tests across the composite, respecting native dependencies. Custom command bodies remain unchanged and must fulfill the required reports; forwarded extra arguments are not yet supported by the default reporting adapter.

Before each run, the adapter removes previous XML from the selected native test directories and their selected coverage XML. Native rerun semantics produce current evidence. JaCoCo finalizers generate coverage, including after failed assertions. Native suites are combined per test task without replacing their test cases, and fresh private JUnit/JaCoCo files enter the shared collector. `dist/manifest.json` links each report and its summary, plan and logs. Native output remains in project build directories; source files and versions are unchanged. Direct tests create no application artifact records and do not run packaging, lint or formatting.

Common pre/post hooks apply; a failed test blocks its success-only post hook. Failed assertions, missing or malformed reports and unmet configured coverage thresholds fail the command while retaining available evidence. `oyzu inspect dist` verifies the resulting bundle. See [direct test reports](direct-tests.md) for output preservation, history, report overrides and host provenance limits. Direct execution is not hermetic, and a local passing test grants no publication or signing authority.

## Captured snapshot builds

Preparation evaluates native models and acquires Maven-layout dependencies through the scoped public repository adapter. Build execution uses the captured repository offline. Private/custom repository connectors and all Gradle plugin sources are not yet supported. Native version projection adds a source-derived snapshot suffix in the private build without editing the checkout. Gradle archive tasks supply filenames and archive versions; supported file kinds include JAR/WAR/EAR, ZIP, TAR and gzip archives. The bundle records their planned versions and digests.

Native Java tests supply JUnit and JaCoCo, with source-free projects omitted. Shared Java quality gates run before successful artifact publication into `dist`. Report injection is shared with direct testing; repository routing, snapshot projection and isolation belong to the captured path. Missing archives, unresolved dependencies and required stage/report failures remain build failures.

## Verification and current limits

`python tooling/test-gradle-direct.py --cli <compiled-path> --gradle-home <gradle-home> --user-home <native-cache>` exercises EX-024 and an included-build test variation with custom JUnit paths, repeated runs, hooks, actual failed assertions, source preservation and bundle inspection. Install `tooling/design-requirements.txt` for the harness. Its optional `--acquire` explicitly provisions fixture dependencies before offline product invocation. CI provisions a checksum-pinned Gradle distribution and runs the probe on Windows, macOS and Linux after compiling the CLI. Both native direct variations passed locally on Windows with Gradle 8.14.3/JDK 17.0.20.1; new cross-host CI results remain pending. Current results are recorded in [implementation status](../implementation-status.md); CI registration alone is not acceptance.

`tooling/test-gradle-preparation.py` separately exercises native repository capture, a fresh offline Gradle home, snapshot archives and report generation. It passed on Windows after the shared reporting extraction, including successful and failed-test reports. It uses an explicit fixture download proxy, not the product executor. The captured Java suite passed at revision 039f83a in [job 110970888463](https://github.com/micahlmartin/oyzu/actions/runs/37046314416/job/110970888463), before the direct-reporting extraction.

Broader Gradle versions, non-Java languages, custom coverage plugins/source ownership, generated test sources and all plugin/composite configurations are not qualified by these fixtures. Native disabled tests and source-free tasks are omitted; unknown test source ownership fails explicitly. Release authorization, private connectors and full authored-scenario coverage remain separate implementation work.
