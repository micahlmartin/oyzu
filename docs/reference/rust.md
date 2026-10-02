# Rust applications, libraries and workspaces

The experimental Rust builder discovers `Cargo.toml` without Oyzu configuration. Cargo owns workspace membership, native package metadata and dependency resolution. Captured builds currently support single packages and contained local workspaces, including libraries, binaries, proc macros and build scripts. Locked crates.io dependencies are captured before execution. Private/alternate registry and Git acquisition, and cross-compilation, remain unfinished.

## Usage and prerequisites

```text
oyzu run list
oyzu run build
oyzu run test
oyzu run lint
oyzu run format-check
oyzu build --plan
oyzu build
oyzu inspect dist
```

Development tasks use already provisioned Cargo, rustfmt and Clippy on the host. Static task listing reads native manifests without executing Cargo. Implicit tasks include install (`cargo fetch --locked`), workspace build/test, Clippy, formatting, formatting checks and archive. `oyzu run format` modifies source; captured builds use read-only `format-check`. Direct test execution now uses native host state and the shared test-only `dist/` report collector; its full native acceptance is pending on the CI toolchains described below. Native Cargo workspace ownership is distinct from separate Oyzu target groups.

Captured builds require Docker and this explicitly provisioned Linux amd64 image:

```text
docker build -f tooling/images/rust.Dockerfile -t oyzu-toolchain/rust:1.94.0-nextest0.9.146-llvmcov0.9.1 .
```

The image supplies Rust 1.94.0, rustfmt, Clippy, LLVM tools, cargo-nextest 0.9.146, cargo-llvm-cov 0.9.1 and Python 3 for the owned artifact adapter. Custom `--image cargo=<image>` profiles must supply these capabilities. Python is now required for compiler-message collection; rebuild older images with the same tag. Tool/image provisioning is explicit and separate from project execution. Oyzu does not download a missing toolchain image.

Docker dependency contexts use the narrower [`rust/cargo` provider](docker-images.md#offline-cargo-registry-context-experimental). Its standard Rust 1.94.0 consumer image needs no Python, nextest or llvm-cov. It validates the original lock/workspace and exports only the native registry, without artifact version projection or report setup. The tool requirements above still apply to ordinary `rust/app` and `rust/library` builds.

The CLI supports Windows, macOS and Linux; that does not imply a native captured-build executor on each host. Current isolated Rust acceptance runs on Linux with Docker. The native artifact probe also runs separately on all three CI hosts. See [implementation status](../implementation-status.md) for revision-specific evidence.

## Preparation and build gates

A checked-in `Cargo.lock` is required. Preparation admits its complete registry inventory before issuing any requests. The current standalone source profile permits crates.io index entries through `https://index.crates.io/` and archives through `https://static.crates.io/crates/`, using the shared host transport and its redirect/size/request limits. A different registry or Git source fails explicitly; there is no direct Cargo network fallback. This fixed public profile is not yet enterprise connector routing or private Cargo authentication.

Each locked registry package must have a SHA-256 checksum. Acquisition requires matching index metadata and archive bytes, then captures the archive and only the locked version's index entry in Cargo's native [local-registry format](https://doc.rust-lang.org/cargo/reference/source-replacement.html#local-registry-sources). Cargo owns extraction, checksum revalidation, resolution and compilation. No project code runs during host acquisition. This avoids implementing a second Cargo resolver or unpacking untrusted crates on the host.

When the lock contains only local workspace packages, preparation writes no registry replacement into the private Cargo home. Locked offline execution still applies. This preserves Cargo's native temporary registry for packaging unpublished workspace dependencies; replacing crates.io with an empty registry prevents that resolution. Single packages with captured registry dependencies retain source replacement.

For a workspace combining local path dependencies and registry inputs, the Rust adapter packages members in dependency order using a private writable copy of the captured registry. Each member runs native `cargo package --locked --offline --allow-dirty --registry crates-io --package <name>` with verification enabled. Cargo generates the normalized archive and its temporary registry entry; the adapter checks their planned name/version and matching SHA-256 before admitting them into that private copy for dependent members. It does not generate Cargo index entries or resolve dependency versions. Local ordering uses native manifest paths, including renamed/optional dependencies. Cyclic local package dependencies and paths without a workspace owner fail planning explicitly.

The original captured registry is immutable. The adapter restores the action's Cargo source configuration and removes its temporary registry on ordinary success or failure; sandbox cleanup owns abrupt termination. Captured packages remain inputs, while only the workspace's planned artifacts can reach `dist/`. Identity/checksum conflicts, a collision with an already captured name/version, an index over 4 MiB or an archive over 128 MiB fail packaging. Repair the native package input or manifest and rebuild; no network fallback or `--no-verify` bypass is used. Registry publication restrictions in native manifests still apply to this explicit crates.io packaging profile; private-registry publication and cyclic-workspace packaging remain unfinished.

Preparation then checks the original lock using native offline Cargo resolution, projects workspace package versions and contained workspace dependency requirements into private manifests, and regenerates the private lock with Cargo. Source files remain unchanged. Unresolved paths and paths outside the captured target fail preparation. Registry packages keep their locked versions and remain inputs; they do not become this workspace's published artifacts.

Prepared input records contain each archive's name, version, digest and size. `digest-only` verification means matching recorded content, not a publisher signature or release authorization. Dependency edges and per-package purpose distinctions are not yet modeled in this inventory. Acquisition currently requires the approved upstreams to be reachable on each fresh preparation; persistent offline acquisition reuse is not implemented. Once preparation succeeds, build execution reads the captured registry with network access denied.

Versions retain each package's base version and use a source-derived suffix, such as `1.2.3-dev.g<source-prefix>`. Different workspace packages retain their individual base versions. These are snapshot versions; they do not establish release eligibility or production trust.

Build execution runs workspace release compilation, native tests with reporting, Clippy, rustfmt checking, native verified crate packaging and final collection. Commands use locked offline inputs and the shared executor denies networking. Build scripts and proc macros execute inside that environment and must work with the captured inputs. The selected compiler host target, `.oyzu-build/target` output root, private Cargo home and offline setting are fixed plan facts; a task environment cannot redirect them. The owned command wrapper installs captured source configuration into that private home so Clippy, nextest and coverage subprocesses inherit it. Custom native source overrides are not supported by this public profile and can fail offline resolution; other native Cargo settings remain in the captured project.

Tests run through `cargo llvm-cov nextest`; native JUnit and Cobertura are retained under `dist/` and indexed with paths, digests and summaries in `dist/manifest.json`. Test execution and coverage generation retain independent failure outcomes. Missing or invalid required reports fail collection. `inspect dist` checks bundle integrity, not release authorization.

Cargo doctests run as part of the same implicit test gate because nextest does not execute them. Prepared native Cargo metadata selects workspace packages with a doctest-enabled target, honoring native `doctest = false`. Each selected package runs `cargo test --doc --locked --offline --package <name>` in the captured workspace. Binary-only targets and registry dependencies do not create doctest invocations. Preparation layout version 5 retains the native doctest selection and relocatable binary inventory; old prepared metadata must be regenerated.

The additional required `doctest.xml` report has one case per Cargo invocation, identified by package name, with bounded native diagnostics. Its suite properties explicitly describe this scope. Counts are invocation outcomes, not the number of individual documentation examples: a doctest-enabled package with no examples can have a successful invocation. Stable Cargo owns assertion semantics; Oyzu does not parse human console text into invented case counts. Native nonzero exits produce failures; launch errors and the 120-second per-package timeout produce errors. Other selected packages still run after a package fails, subject to the enclosing action timeout. Each invocation starts with fresh report output; prior success cannot satisfy a failed launch.

Doctest failures block artifact collection even if nextest and coverage generation succeed. Inspect the named report and native diagnostics to repair the example, compilation or provisioned rustdoc/linker environment, then rebuild. Task replacement retains this additional report obligation. The existing coverage report measures nextest execution only: doctest coverage and individual doctest JUnit cases remain unfinished, as do every custom nextest configuration and broader framework support.

Compilation, tests, lint, formatting and archive verification are required gates. Their failures block final artifacts. Native logs and available reports explain the failed stage. Repair the native input and rerun; an earlier successful bundle is retained under `.oyzu/history`, not reused as this build's successful output.

## Direct test evidence

`oyzu run test` produces nextest JUnit, native LLVM Cobertura coverage and Cargo doctest invocation reports, followed by `oyzu inspect dist`. The real app/workspace reporting step passed on Windows MSVC, Linux and macOS in [run 37049720609](https://github.com/micahlmartin/oyzu/actions/runs/37049720609) at bfa2b7c. This verifies the direct reporting step, not the entire still-running workflow. The local Windows GNU Rust 1.94.0 compiler lacks `profiler_builtins`; its earlier coverage failure remains a toolchain limitation.

Provision Python 3.11+, Rust/Cargo with its `llvm-tools-preview` component and compiler profiler runtime, cargo-nextest 0.9.146 and cargo-llvm-cov 0.9.1. Windows uses an MSVC compiler and its native linker/SDK for coverage; the local GNU compiler is not supported by this flow. Dependencies must already be installed for locked offline Cargo resolution. Tools are not installed by the product. Missing LLVM tools fail before llvm-cov can request a download; explicit `LLVM_COV`/`LLVM_PROFDATA` paths can identify provisioned binaries.

After shared task admission, Cargo's native `metadata --no-deps --locked --offline` supplies workspace membership and doctest-enabled packages. Static `run list` never invokes Cargo. The exact implicit `cargo test --locked --workspace` command receives automatic instrumentation. Arbitrary task replacements retain their bodies and required JUnit/coverage/doctest obligations; they do not waive evidence collection. No Oyzu build/test configuration is needed for the default flow.

A temporary nextest profile inherits the project's `default` configuration and supplies fresh JUnit output; the native `.config/nextest.toml` is not edited. This uses nextest's [native profile inheritance](https://www.nexte.st/docs/configuration/). Project coverage/reporting state is not reused: compilation and coverage use private temporary target directories. Native nextest may retain its ordinary store output on the host. Extra command arguments go to the nextest invocation; doctest selection remains the metadata-selected workspace packages. Alternate nextest profiles, arbitrary Cargo test argument compatibility and filtered doctest selection are not implemented by this initial direct adapter.

The cross-platform Python runtime is shared by captured and direct execution. It runs nextest, attempts coverage generation independently of a failed test, and runs the existing stable Cargo doctest adapter. Any failed phase fails the task. Doctest reports count package invocations, not individual examples, and doctest execution is not included in nextest coverage. Custom native settings, build scripts and proc macros retain their normal execution semantics. Direct reports disclose host execution and unverified tool identity; Cargo's offline flags are not a host network sandbox. See [direct test bundle boundaries](direct-tests.md).

`python tooling/test-rust-direct.py --cli <compiled-path>` uses EX-021 and EX-022: implicit task listing, default-profile settings, native JUnit/Cobertura, successful and failed tests, independent doctest failure, unchanged source/configuration and bundle inspection. It requires `tooling/design-requirements.txt` for report-contract validation. CI explicitly provisions the pinned reporters with `tooling/provision-rust-reporting.py` after downloading the compiled CLI. The three-host direct-reporting steps passed at bfa2b7c; broader native profile and argument combinations remain outside that probe.

## Artifact selection and collection

Each selected package produces a native `<package>-<snapshot-version>.crate`. Each enabled binary additionally produces `<binary>-<snapshot-version>-<compiler-host>`. Manifest names use the existing scoped `crate-` and `bin-` identities. A library-only package has a crate artifact without an invented executable. Duplicate binary names currently fail planning instead of colliding in the output bundle.

Cargo's resolved features determine whether a binary with `required-features` is enabled. An unmet feature gate omits that binary from the artifact plan; enabling the feature through native defaults includes it. No Oyzu-specific switch or new build YAML is needed. This increment does not add a feature matrix or new CLI feature-selection syntax. Cargo documents [required-features](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-required-features-field) as native target selection.

The build adapter consumes Cargo's [JSON compiler messages](https://doc.rust-lang.org/cargo/reference/external-tools.html#json-messages). It matches opaque package IDs and binary target names against a prepared inventory and takes executable paths from native `compiler-artifact` events. It accepts fresh Cargo cache results as well as newly compiled binaries, requires a successful native process and build-finished event, rejects missing/unplanned/conflicting outputs and verifies each executable is a regular file contained in the target root. It does not guess executable filenames or publish intermediate test binaries.

The adapter clears its reserved staging directory before invoking Cargo. Only after all expected binaries are verified does it stage them for the shared final collection step. This prevents a previous executable from satisfying a disabled feature or failed compilation. Native source archive contents are checked in the scenario harness to exclude private `.oyzu-build` files. Cargo does not automatically exclude arbitrary custom output directories; the captured profile fixes its private output location. Exhaustive custom Cargo include/exclude packaging behavior remains unverified.

Task overrides and pre/post hooks retain their existing shared owners. Replacing a native build task must still satisfy its planned artifact obligations; an override does not waive collection checks. No registry publication, signing or promotion is performed by this builder increment.

## Verification and remaining work

```text
python tooling/test-rust-artifacts.py --cargo <provisioned-cargo>
python tooling/test-rust-doctests.py --cargo <provisioned-cargo>
python tooling/test-rust-packaging.py --cargo <provisioned-cargo>
```

This native probe checks gated binaries disabled/enabled by Cargo defaults, exact executable bytes, launching collected executables, fresh cache results, source archive exclusions, missing outputs, redirected output paths and compiler failures. The [Rust example variants](../../examples/builds/rust-app/variants/) supply the native inputs. Rust planner checks cover per-package versions, feature resolution and fixed output facts.

The doctest probe runs real stable Cargo against a local workspace, checking successful examples, assertion/compile failures, independent package outcomes, missing-tool stale-report replacement and unchanged inputs. Its `--cargo` option accepts a command on `PATH` or an executable path; relative paths are anchored before changing into the fixture, and symlinks retain the Cargo invocation name required by rustup. Unix probes additionally execute through a temporary Cargo symlink. Windows verification passed with Rust 1.94.0; the local GNU toolchain required its provisioned linker setting in `RUSTDOCFLAGS` as well as `RUSTFLAGS`. [Run 36971869232](https://github.com/micahlmartin/oyzu/actions/runs/36971869232) passed all three CLI jobs, including the corrected native doctest probe and local-workspace packaging test. The captured Linux suite requires failed doctests to block artifacts despite successful nextest results; isolated doctest acceptance is still pending.

The Rust test suite also gives native Cargo a fresh home and an acquired fixture registry, then runs tests, Clippy and verified packaging offline. It checks unchanged source lock bytes and rejection of corrupted archives from a fresh cache. Acquisition checks reject unsupported sources before fetching and mismatched index/archive checksums.

The mixed-packaging probe uses native Cargo to create a registry fixture, then packages a three-member snapshot workspace against it with no downloads. It checks archive contents, locked external dependencies, repeatability, source/registry/configuration preservation, rejected index tampering and failures during native verification. GitHub Actions runs it on each CLI host. The checked-in EX-022 registry variation additionally exercises the captured CLI build, reports, delivered binary and failure-gated collection; passing the host probe alone does not prove that isolated scenario.

The compiled-CLI Linux scenario suite additionally builds applications, libraries and local workspaces, executes delivered binaries, checks snapshot crate contents, measures application coverage, verifies repeatability and confirms failed tests/formatting block collection. The [registry example](../../examples/builds/rust-app/variants/registry/) adds a locked public dependency with no Oyzu configuration; its CI checks require retained dependency identity, versioned outputs, JUnit/coverage, unchanged inputs and rejection of a modified lock checksum. New registry and feature-gate scenarios must pass that suite before being described as verified captured behavior. Native host checks alone do not prove isolated build acceptance.

Remaining work includes private/alternate Cargo registries, Git sources and credentials, persistent acquisition reuse, full feature/profile/platform matrices, doctest coverage/individual-case reporting and broader test-framework support, custom task artifact production, release policy, publication, remote caching and complete authored-scenario acceptance. The build goal remains broader than this implementation increment.


Native binary capture now identifies a planned binary by its target-relative Cargo manifest path and target name. This preserves ownership when a prepared target rooted at `/workspace` executes under `/workspace/<target>` in a mixed repository. Cargo package IDs remain opaque; Oyzu does not rewrite or parse them. The change fixes the nested-target failure observed by the live logging acceptance probe. Inventory version 2 records relative manifest paths; the runtime retains version 1 support for existing native adapter probes.
