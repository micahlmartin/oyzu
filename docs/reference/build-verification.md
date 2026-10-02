# Running builder acceptance checks

The repository's captured-build harness executes the compiled Oyzu CLI against native project fixtures, inspects actual artifacts and reports, and checks expected failures. It covers the implemented profiles recorded in [implementation status](../implementation-status.md). A passing harness run does not establish acceptance of every case in the 58 authored design-contract scenarios.

## Select and run checks

Use Python 3.12 with `tooling/design-requirements.txt` installed to inspect the current groups without running builds:

```text
python tooling/test-build-scenarios.py --list-suites
python tooling/check-build-suites.py
```

The inventory check verifies unique registrations, conventional acceptance entry points, matching CI matrix coverage, compilation dependency and the aggregate gate. It neither provisions tools nor verifies native product behavior.

For real builds, supply a compiled CLI and provision the required toolchains. Captured-build acceptance currently runs on Linux with Docker and the toolchain definitions in `tooling/images/`. Node/core/Docker checks also use host Node 22/npm; Java native adapter checks require host JDK 17. The provisioning script uses network access to obtain test tooling and images. Go/Docker suites install the repository's explicit BuildKit AppArmor profile using `sudo apparmor_parser`; use a suitable Linux test machine. This is acceptance-fixture setup, not Oyzu's tool-installation feature or an implicit network fallback during a product build.

From the repository on that Linux test machine:

```bash
cargo build --locked
bash tooling/build-scenario-tools.sh provision docker
export OYZU_BUILDKIT_APPARMOR_PROFILE=oyzu-buildkit
.ci-python/bin/python tooling/test-build-scenarios.py --suite docker --cli target/debug/oyzu --evidence-dir docker-evidence
```

For suites with standalone native adapter probes, `bash tooling/build-scenario-tools.sh native <suite>` runs those additional checks. The Helm CLI probe expects the compiled binary at `cli/oyzu`, matching CI; copy your binary there before running that probe. The ordinary captured-build harness accepts any explicit `--cli` path. Native probes supplement captured builds; they cannot substitute for them.

Omitting `--suite` selects `all` and retains the original full-run ordering. Provisioning/native-probe scripts also accept `all`. A single-suite selection uses the same checks and assertions as the full run. Unknown suites and missing `--cli` arguments fail rather than selecting a fallback.

| Suite | Included checks and toolchain dependencies |
| --- | --- |
| `core` | Concurrency, cross-target task prerequisites and hook failures, Go-to-Node materialization, baseline Node/Go/Python artifacts, repeatability and isolation; includes npm, Go, pip, uv and Poetry tooling |
| `node` | Native overrides, dependency/preflight failures, npm/pnpm/Yarn, workspaces, lint/format, Jest, Vitest and Mocha |
| `python` | Test/report defaults, application archives, legacy packaging and quality selection; includes native Python adapter probes |
| `go` | Libraries, workspace/cgo, registry modules and selected multi-binary image assembly; includes Docker tooling |
| `rust` | Native Cargo application/workspace/registry, packaging, tests and quality profiles |
| `java` | Ant, Maven and Gradle captured builds plus native metadata/reporting adapter probes |
| `helm` | Chart packaging, rendering, native suites and subchart variants plus native adapter probes |
| `docker` | Dockerfile images, provisioned inputs, aliases, ARG defaults, quality, Go artifact assembly and Vite directory materialization; includes Go and npm/Node quality tooling |

The catalog is owned by `tooling/test-build-scenarios.py`; native assertions remain in `tooling/build_scenarios/`. New conventional `verify(root, base, invoke, validate, source_files, verified)` entry points must be registered. Keep helper assertions with their owning group and update provisioning when a composition check needs another builder's toolchain. No production builder rules belong in this harness.

## CI and evidence

The workflow builds the CLI on Linux, macOS and Windows first. Linux captured-build jobs download that run's compiled Linux binary, then run the eight suites with up to four concurrent jobs. Each job provisions only its required toolchain families. `fail-fast: false` allows unrelated suites to finish after one fails. The `Captured build suite acceptance` gate fails if any suite fails, is cancelled or is skipped. Three-host CLI/task checks and the separate isolated-worker job remain distinct checks with their own scope.

CI uploads each suite under `oyzu-build-evidence-<suite>`, replacing the former single `oyzu-build-evidence` artifact name. Evidence directories contain numbered build invocations, their available `dist/` bundles, and a `summary.json` identifying the suite, selected groups, completed groups, verified assertions and success/failure. An exception preserves a failed summary with the active group; previously completed checks do not make an incomplete suite successful. Process termination can prevent final summary writing. The harness does not turn missing reports into passing evidence.

An evidence directory must be new; choose a fresh path for reruns. Inspect the failing invocation and native logs, correct the underlying behavior or fixture, and rerun the affected suite. Do not weaken assertions or mark authored scenarios complete based on inventory checks. Retained artifacts contain build logs and sample outputs and are kept for 14 days in CI; fixtures must not contain real credentials.

Local verification of this suite split includes inventory and shell/Python syntax checks, an AST comparison preserving existing baseline assertions and embedded source strings, retained provisioning/probe command checks, and failed-summary behavior after a missing executable. New Linux suite execution remains subject to the revision-specific CI evidence in implementation status. Windows/macOS execution of the full captured-build matrix and the remaining authored cases are still required work.
