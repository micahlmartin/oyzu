# Running builder acceptance checks

The repository's captured-build harness executes the compiled Oyzu CLI against native project fixtures, inspects actual artifacts and reports, and checks expected failures. It covers the implemented profiles recorded in [implementation status](../implementation-status.md). A passing harness run does not establish acceptance of every case in the 58 authored design-contract scenarios.

## Select and run checks

Use Python 3.12 with `tooling/design-requirements.txt` installed to inspect the current groups without running builds:

```text
python tooling/test-build-scenarios.py --list-suites
python tooling/check-build-suites.py
```

The inventory check verifies unique registrations, conventional acceptance entry points, matching CI matrix coverage, compilation dependency and the aggregate gate. It neither provisions tools nor verifies native product behavior.

For real builds, supply a compiled CLI and provision the required toolchains. Captured-build acceptance currently runs on Linux with Docker and the toolchain definitions in `tooling/images/`. Node/core/Docker/dependencies checks also use host Node 22/npm; Java native adapter checks require host JDK 17. The provisioning script uses network access to obtain test tooling and images. Core/Go/Docker/Python/dependencies suites install the repository's explicit BuildKit AppArmor profile using `sudo apparmor_parser`; use a suitable Linux test machine. This is acceptance-fixture setup, not Oyzu's tool-installation feature or an implicit network fallback during a product build.

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
| `core` | First the authored five-target mixed-monorepo build, then concurrency, explicit target selection and dependency closure, cross-target task prerequisites and hook failures, Go-to-Node materialization, baseline Node/Go/Python artifacts, repeatability and isolation; includes npm, Go, pip, uv, Poetry, Helm and Docker tooling |
| `node` | Native overrides, dependency/preflight failures, npm/pnpm/Yarn, workspaces, lint/format, Jest, Vitest and Mocha |
| `python` | Test/report defaults, application archives, legacy packaging and quality selection; includes native Python adapter probes |
| `go` | Libraries, workspace/cgo, registry modules and selected multi-binary image assembly; includes Docker tooling |
| `rust` | Native Cargo application/workspace/registry, packaging, tests and quality profiles |
| `java` | Ant, Maven and Gradle captured builds plus native metadata/reporting adapter probes |
| `helm` | Chart packaging, rendering, native suites and subchart variants plus native adapter probes |
| `dependencies` | Offline Docker dependency contexts for pip, uv, Poetry, npm, Yarn Classic, pnpm, Go and Cargo; includes their consumer runtimes and BuildKit, plus native store/export probes |
| `docker` | Dockerfile images, provisioned inputs, aliases, ARG defaults, quality, Go artifact assembly and Vite directory materialization; includes Go and npm/Node quality tooling |

The catalog is owned by `tooling/test-build-scenarios.py`; native assertions remain in `tooling/build_scenarios/`. New conventional `verify(root, base, invoke, validate, source_files, verified)` entry points must be registered. Keep helper assertions with their owning group and update provisioning when a composition check needs another builder's toolchain. No production builder rules belong in this harness.

The `core` suite starts with `mixed-monorepo`, the first complete EX-030 demonstration. It copies the checked-in project without changing its five-target `build.yaml`, lists grouped tasks and invokes one `oyzu build`. Assertions require successful build/test/lint/read-only-format stages for every target, Python wheel/sdist/application outputs, a Node application directory, a Go binary embedded byte-for-byte in the OCI image, and a versioned Helm chart plus rendered deployment. The manifest must retain JUnit for every target and application coverage for Python/Node/Go, and `oyzu inspect dist` must verify the bundle. Provision `core` as above with Helm and Docker included. The Linux captured result is pending; registration and native host checks do not establish a passing demonstration.

EX-030's first proof is separate from its advanced negative cases, broader host qualification and proposed image-to-chart value binding. The current `depends_on: [image]` orders the chart after image packaging; it does not replace native chart values with the image digest. The check preserves that distinction and does not mark the entire authored scenario complete.

The `dependencies` suite separates package-store integration from the Docker platform-matrix checks. Run it explicitly when changing providers:

```bash
bash tooling/build-scenario-tools.sh provision dependencies
bash tooling/build-scenario-tools.sh native dependencies
export OYZU_BUILDKIT_APPARMOR_PROFILE=oyzu-buildkit
.ci-python/bin/python tooling/test-build-scenarios.py --suite dependencies --cli target/debug/oyzu --evidence-dir dependency-evidence
```

Previously these five context groups ran inside `--suite docker`; that selection now runs image/platform and Node-application groups only. Use both suites or `all` to retain the previous coverage. The default `all` order and all existing assertions remain unchanged. Dependencies use native Linux consumer images and do not need the Docker suite's ARM emulation. Both suites remain required by the aggregate CI gate; this split lets package failures surface without first executing the long platform matrix.

## CI and evidence

The workflow builds the CLI on Linux, macOS and Windows first. Linux captured-build jobs download that run's compiled Linux binary, then run the nine suites with up to four concurrent jobs. Each job provisions only its required toolchain families. `fail-fast: false` allows unrelated suites to finish after one fails. The `Captured build suite acceptance` gate fails if any suite fails, is cancelled or is skipped. Three-host CLI/task checks and the separate isolated-worker job remain distinct checks with their own scope.

Explicit Java quality, Hadolint and yamlfmt provisioning shares `tooling/provisioning.py`. Downloads make at most three attempts for transient transport failures and HTTP 429/500/502/503/504, with one- and two-second backoff, a 60-second socket timeout per attempt and the existing asset-specific byte limits. A truncated response with a declared content length is discarded before retrying. Authorization/not-found errors and oversized responses are terminal. Pinned SHA-256 verification remains mandatory in each owning provisioner; digest mismatches are not retried or accepted. The timeout is per socket operation, not a total invocation deadline. This supports CI/toolchain setup only and adds no product tool-installation or build-time internet access. The three-host task job exercises loopback retry/size/failure tests before provisioning.

CI uploads each suite under `oyzu-build-evidence-<suite>` (including `oyzu-build-evidence-dependencies` for package contexts), replacing the former single `oyzu-build-evidence` artifact name. Evidence directories contain numbered build invocations, their available `dist/` bundles, and a `summary.json` identifying the suite, selected groups, completed groups, verified assertions and success/failure. An exception preserves a failed summary with the active group; previously completed checks do not make an incomplete suite successful. Process termination can prevent final summary writing. The harness does not turn missing reports into passing evidence.

An evidence directory must be new; choose a fresh path for reruns. Inspect the failing invocation and native logs, correct the underlying behavior or fixture, and rerun the affected suite. Do not weaken assertions or mark authored scenarios complete based on inventory checks. Retained artifacts contain build logs and sample outputs and are kept for 14 days in CI; fixtures must not contain real credentials.

Local verification of this suite split includes inventory and shell/Python syntax checks, an AST comparison preserving existing baseline assertions and embedded source strings, retained provisioning/probe command checks, and failed-summary behavior after a missing executable. New Linux suite execution remains subject to the revision-specific CI evidence in implementation status. Windows/macOS execution of the full captured-build matrix and the remaining authored cases are still required work.

The Docker captured suite now provisions ARM emulation explicitly through a pinned setup-qemu action before building Go and Node ARM images. The provisioner checks actual runtime architecture without network access. This is CI infrastructure, not product tool/emulator installation. Local execution of this suite needs equivalent pre-provisioned execution support. The two producer platform-matrix invocations allow up to 2,400 seconds in the test harness for emulated compilation; product acquisition/action limits remain unchanged. See [toolchain platform verification](toolchain-platforms.md).

The isolated worker job additionally compiles and explicitly runs the Rust [generated-definition conformance test](../container-assembly.md#verification) after provisioning. It checks real reproducible OCI export, payload bytes/ownership and launch metadata with no source Dockerfile; it does not establish application packaging or smoke-test support. Its logs and archives are retained under `worker-recipe-evidence` in the worker artifact, alongside `worker-evidence`. Ordinary Rust tests compile this check but skip its Docker execution.

The Python suite additionally provisions the shared BuildKit/converter image and runs the authored [Python container variant](python-containers.md). It checks exact archive materialization, default and custom provisioned bases, numeric user/workdir/argv overrides, OCI metadata and versioning, native reports/quality, repeated identities, failure blocking and incompatible runtime rejection. This group passed in [Python job 110898263768](https://github.com/micahlmartin/oyzu/actions/runs/37024166958/job/110898263768) at 69b4d04. Worker-only conformance does not substitute for this application integration.

The isolated worker job additionally runs the internal dependency-context conformance test after base provisioning. It exercises real RUN/COPY consumption, mount write rejection, exact subtree bytes, sibling exclusion and repeatable output. Evidence stays under `worker-recipe-evidence/dependency-context`. This is executor transport qualification; custom package-manager acquisition remains a separate acceptance requirement.

The dependencies suite's `docker-dependencies` group adds actual public pip preparation in a provisioned Python 3.13 base, plus native uv/Poetry lock export and offline installation in provisioned Python 3.12 manager images. It checks real imports during RUN, package/runtime evidence, OCI/JUnit/quality output, repeated identities, implicit task discovery, explicit selection after ambiguity, rejected lock hashes, excluded development groups, and rejection of an attempted build-time internet download. This group is registered after CLI compilation; its first native results are pending. It does not claim private connector, secret-canary, other-ecosystem or full EX-058 acceptance.

`tooling/test-python-context.py --manager uv` (or `poetry`) probes native runtime-only export, unchanged/stale locks, offline hashed installation/import and rejection of modified wheels. Provision Python 3.12, the corresponding manager and the Poetry export plugin before invoking it. Native fixture lock creation and wheel download use public network access; the export and install operations are offline. The dependencies suite runs these probes in its provisioned manager images before captured builds. They also passed independently on Windows with provisioned uv 0.12.21 and Poetry 2.5.1, but do not by themselves establish broker or container isolation.

The dependencies suite also registers `docker-npm-context`, using the already-provisioned npm 11.11.0/Node 22 image. It requires locked broker acquisition in the consumer runtime, offline cache seeding and `npm ci`, exact runtime files with development packages omitted, captured package/runtime evidence, OCI/JUnit/quality, repeatability and rejected lock integrity. Native CI results are pending. `tooling/test-npm-context.py` separately exercises the acquisition adapter and tarball-only native replay; it passed on Windows with npm 11.11.0/Node 24.14.1. Dependencies-suite provisioning installs the pinned fixture npm from `tooling/images/node-npm` for this host probe, and the native phase selects that entrypoint explicitly. Fixture lock creation/downloads use the public registry; captured replay uses new empty caches offline. Neither fixture provisioning nor the spool test transport establishes private connector or full EX-058 acceptance.

`docker-yarn-context` additionally provisions the Yarn Classic 1.22.22/Node 22 toolchain and requires native locked mirror preparation, offline installation/import, exact scoped/transitive image files, lifecycle suppression, snapshot OCI/JUnit/quality, repeatability and altered-lock rejection. Its native CI result is pending. The suite's native phase runs `tooling/test-node-registry-acquisition.py --manager yarn --context --native-cli tooling/images/node-yarn/node_modules/yarn/bin/yarn.js`, both normally and with `--resolutions`. Those probes passed on Windows with Node 24.14.1, including fresh-cache direct Yarn consumption, unchanged mirror/locks, native tests and corrupted mirror rejection. Existing acquisition/replay tests remain enabled; `--context` is a test harness option, not public CLI configuration.

The first pip-context attempt at revision 5c8eaf6 failed in the harness before its image build: `run list` returned human-readable output to a JSON-only invocation helper. The pip and npm checks now explicitly request `run list --json`, matching existing suite conventions; both corrected task-list calls passed against the compiled CLI on Windows. The original failed job remains [110912572149](https://github.com/micahlmartin/oyzu/actions/runs/37028866911/job/110912572149). It verified preceding Docker groups but provides no package-context build acceptance.

`docker-pnpm-context` provisions pnpm 10.11.0/Node 22 and requires ordinary and patched projects to consume the exported native store offline, retain exact image/patch evidence, remove temporary store files, produce snapshot OCI/JUnit/quality, repeat identities and reject altered lock integrity. Native CI results are pending. The dependencies native phase runs the registry probe with `--manager pnpm --context`, both normally and with `--patches`. Windows pnpm 10.11.0/Node 24.14.1 probes passed native store export/replay, empty graphs, tests after store removal, unchanged locks, stable repeated exports and corrupted-store rejection. Provisioning remains separate from Oyzu tool installation, and these host probes do not establish Docker isolation.

`node --test tooling/test-pnpm-store.mjs` exercises the export boundary's rejection of unqualified versions, configuration files, side-effect indices and directory links/junctions into unrelated state. These three checks passed on Windows and run in the dependencies suite's native phase. They complement native pnpm replay rather than simulating package resolution or proving container output.

`docker-go-context` uses the already-provisioned standard Go 1.24 image, without the modulezip adapter. It requires captured public module acquisition, read-only native cache compilation and execution in the build stage, an exact binary-only scratch image, versioned OCI/JUnit/quality, repeatability and rejected checksum before actions. Native container results are pending. The existing three-host `tooling/test-go-metadata.py` probe now additionally copies only the prepared module subtree into two independent stores, compiles/runs offline with fresh compiler caches, and checks identical binaries and unchanged source/store bytes. That extended probe passed on Windows with Go 1.24.13; it does not by itself establish a read-only container mount or final-image startup.

`docker-rust-context` additionally provisions standard Rust 1.94.0 and Debian bookworm slim images. It requires actual native registry preparation without nextest/llvm-cov, unchanged native workspace versions, offline compilation and execution in the build stage, runtime-only image output, versioned OCI/JUnit/quality, repeatability and checksum rejection before actions. Its native container result remains pending. The Rust acquisition test, run during CLI verification on each host, now also compiles/runs against two detached registry copies with fresh native homes and checks unchanged locks/store bytes; that probe passed on Windows with Rust 1.94.0. It does not replace the captured-build check or claim final-container startup.

The a653ea8 dependency job [110942213239](https://github.com/micahlmartin/oyzu/actions/runs/37037714359/job/110942213239) completed with failure after passing the pip context checks. Its two successful uv image builds had equal plan digests but different OCI digests. Inspection of the retained images found only `uv_cache.json` and its `RECORD` hash changed: native uv recorded a varying input-wheel modification timestamp. Original evidence remains retained under artifact 11240339977; this is not a successful dependency-suite run, and later groups were not reached.

The executor now normalizes its private input copies before native execution. Its Rust test varies input times and verifies fixed file/directory times, unchanged content identity and unchanged originals, including a read-only Windows file. The Linux worker conformance test additionally records source/package/directory mtimes observed inside RUN and requires the fixed epoch plus repeated OCI identities after source times change. `test-python-context.py --manager uv` now supplements its existing pip replay checks with two real native uv installs against fixed-time copied stores; it checks equal installed bytes and retained native cache metadata. The Windows uv 0.12.21 probe passed. These host checks do not replace the pending corrected container build.
