# Builder implementation status

The active objective is the complete builder system, all applicable sample scenarios passing through real Oyzu behavior, snapshot artifacts, and CI that builds the CLI before running scenario verification. Tool installation is excluded. The checkpoints below do not redefine completion around a subset.

## Checkpoint 1: native discovery and development tasks

Implemented in Rust:

- CLI project selection, JSON discovery and grouped task listing.
- Static native discovery for Python managers, Go, Cargo, Node managers, Maven, Gradle, Ant, Docker and Helm.
- Explicit build targets, native task commands, visible unavailable integrations, same-name TOML overrides and pre_/post_ ordering.
- Unknown/ambiguous task, conflicting native manager, escaping target path and cyclic task-dependency errors.
- Real development execution through native tools; no assertion of hermeticity or managed authorization.

Verification at this checkpoint: five Rust integration tests, including discovery across 22 existing project roots; black-box CLI checks of native npm lifecycle, grouped task ambiguity, all hook failure positions, and a real Node native build/test. These exercise parts of EX-007/008/009/018, not complete end-to-end scenario acceptance. Rust formatting and lint are mandatory checks. Host-specific CI evidence is tracked through the workflow, not inferred from local success.

The GitHub Actions workflow builds/tests the CLI on Linux, Windows and macOS and uploads the compiled binaries. A dependent matrix job downloads those binaries and runs the task scenario checks. The full build-scenario matrix is still to be implemented; a green task workflow must not be reported as all 58 scenarios passing.

## Outstanding implementation requirements

| Requirement | State |
| --- | --- |
| Immutable source capture, deterministic plan, graph/platform expansion and actual build command | Captured source, serial plans and initial Node/npm and Go build command; graph/platform expansion pending |
| Dependency preparation and private-registry credential isolation for every native manager | Initial public Python wheel acquisition and broker boundary tests; all-manager/private-source integration pending |
| Capability-enforced executor, cancellation/process containment and offline build actions | Docker offline executor integrated; full boundary/cancellation verification pending |
| Build/test/lint/read-only formatting orchestration with native ownership and reports | Node/npm and Go verified in Linux CI; Python/pip, uv and Poetry integration under native verification |
| Native snapshot version projection and verified dist artifacts/manifests, including failures | Node packages and Go binaries verified; Python wheel/source archive checks added |
| Container convenience packaging, Dockerfile contexts/materialization, multi-platform outputs | Discovery only |
| Helm dependency capture, chart output and image digest bindings | Discovery only |
| Java multi-module, Go workspace/cgo, Rust features, Node workspaces and all Python variants | Discovery only; native build integration pending |
| OCI action cache, producer evidence and snapshot publication/retry | Pending |
| Managed policy, source-control facts, service-test/sandbox negative cases and broker behavior | Pending |
| Complete scenario runner with native registry fixtures and accurate per-scenario evidence | Pending |
| Cross-host verified full builds using the compiled CLI in GitHub Actions | Three-host task workflow; first Linux container build checks added |

The implementation must not read scenario expectation JSON as instructions for manufacturing outputs. Native source/configuration determines behavior. Existing authored scenarios retain pending-implementation status until their actual acceptance cases, including negatives, are exercised. No fixture is marked passing merely because a CLI command exists.

## Next checkpoint

Initial executor and report helpers are also checked in as foundations for OEP-0007 and OEP-0012. The Docker helper requires a provisioned Linux image, resolves its identity, disables container networking and mounts only supplied workspace/output directories. JUnit, Go test-event conversion and LCOV/Go coverage summaries have focused parser tests. Both helpers are now wired into the initial `oyzu build` command. Container execution, cancellation, mount containment and complete report validation still require further acceptance verification; compilation and parser tests do not establish those guarantees.

The initial Node/Go source-to-snapshot path has passed Linux CI. Python dependency preparation and packaging are now integrated; next establish native CI evidence and expand the same engine across the remaining managers. Keep committing incremental verified changes.

## Checkpoint 2: captured-source primitive

Source copying now creates a separate content-identified tree, excludes common dependency/output stores, detects changed file contents during capture, rejects nonportable/colliding paths and refuses symlinks rather than following them into host files. Existing or source-nested destinations fail. Local tests prove equal content at different checkout paths shares an identity and later source edits do not change captured bytes. A Unix-only escaping-symlink test is included for CI.

This is not yet the complete source isolation contract: allowed internal links, cross-host executable-mode normalization, repository ignore rules, race-resistant handle-based filesystem access remain required. These limitations are not treated as passing the sandbox examples. The full builder catalog remains pending.

## Checkpoint 3: first captured-source build path

`oyzu build` now constructs a plan from captured source and pre-provisioned Docker image identities, then runs serial offline actions for dependency-free npm packages and simple Go applications. `oyzu build --plan` performs capture/preflight/planning without executing project commands. `oyzu inspect dist` checks recorded content digests; it does not authenticate a producer or grant release eligibility.

The plan records native commands, inferred build/test/lint/format-check tasks, hooks, target paths, source and toolchain identity, snapshot artifact names and report intents. Semantic records use RFC 8785 encoding. Build-time shell defaults follow the Linux executor, even on a Windows CLI host. Native npm lifecycle remains npm-owned. Go formatting checks do not edit the source. The Go application fixture was formatted to meet that actual check.

Bundles contain plans, execution context, native stdout/stderr, declared artifacts, JUnit and coverage reports and a success/failure manifest. Existing Oyzu bundles are retained under `.oyzu/history`; an unrelated existing `dist` is rejected. Local and CI-indicated runs remain unverified for production. The engine never pulls toolchain images during a build.

Unit tests cover location-independent plans, toolchain digest binding, hook ordering, executor shell selection, semantic encoding and detection of changed artifact bytes. A compiled-CLI Linux CI job now exercises real Node/Go artifact contents, reports, failure manifests, repeated builds, source preservation and actual container network/filesystem/environment restrictions. Its result is authoritative for those cases; broader examples are still pending.

Remaining limits include dependency acquisition, non-Node/non-Go packaging, matrices, materialization, container packaging, cache/publication, managed policy, native macOS/Windows executors, process cancellation, strict output quotas, complete report validation, authentic source-control evidence and recovery from every interrupted/faulted finalization. Inferred checks that are unavailable do not constitute passed checks. npm workspaces and projects declaring dependencies fail explicitly for now. This checkpoint does not satisfy the full objective.

### Verified CI evidence

[Run 36829019392](https://github.com/micahlmartin/oyzu/actions/runs/36829019392) passed at commit `bab3b17196434d0a408c076b6f7f0a602d9ed4b2` on 2026-10-01. All three host CLI build/test/lint jobs and compiled-CLI task jobs passed. The Linux captured-source job ran the downloaded release CLI against real Node and Go source fixtures. It verified package metadata/content, execution of the delivered Go binary, identical artifact digests on repeated builds, JUnit/coverage records, unchanged source, retained prior bundles, tamper detection, failed tests, failed nonmutating formatting, build hooks and actual container network/root-write/socket/environment restrictions.

These are partial acceptance cases for EX-009/016/018/036/042. The authored scenarios retain pending-implementation status because their complete requirements, and the remaining scenario catalog, are not yet satisfied.

## Checkpoint 4: Python acquisition and build integration

The initial Python/pip profile now has a scoped fetch broker and native wheel resolution before the final plan. The resolver runs with container networking disabled and reaches approved PyPI index/blob routes through a private, per-acquisition file channel and a loopback adapter inside that container. Host-side requests validate each redirect and bind authorization to the matching source. No upstream authorization header, host listening socket, or general CONNECT proxy is supplied to the resolver. The channel is absent from subsequent build actions.

Rust tests use real synthetic HTTP servers to check route denial, redirect authorization, non-forwarding of credentials across sources and private-channel response handling. The initial default sources are public PyPI routes; managed enrollment, user registry configuration, private-source entitlement, broker cancellation hardening and the complete cross-platform acquisition security contract remain pending. The tested synthetic credential boundary does not establish all private-manager scenarios.

The build profile currently targets static PEP 621 pure-Python projects through a pre-provisioned `python:3.12-slim-bookworm` image. Native pip resolves wheels and captures package identities, edges and digests; source-owned pip requirement hashes are checked separately. Offline actions install the captured inputs, project a PEP 440 snapshot version, invoke the native build backend, run pytest/JUnit/Cobertura and package wheels/source archives. Source-archive transport metadata is normalized for repeatability. Dependency evidence is included in the bundle. The new Linux CI case is authoritative for end-to-end support; it is not inferred from compilation or broker unit tests.

uv/Poetry lock semantics, requirements-only application packaging, legacy/dynamic metadata, source distributions requiring build preparation, native extensions, dependency cache reuse and all other manager adapters are still required. Unsupported forms must not silently become a different manager or bypass approved routes.

The uv profile uses the pre-provisioned `ghcr.io/astral-sh/uv:0.12.21-python3.12-trixie-slim` image. It invokes `uv export --locked --offline` with the image interpreter, verifies exported native hashes during acquisition, records the original lock digest and constrains all captured runtime/test packages to that export. `uv build` and `uv run --offline --no-sync` execute against the prepared environment; the source lock is not rewritten for snapshot version projection. Custom sources, absent locks and unsupported workspace forms are not silently converted to pip projects. The uv CI case includes native stale-lock rejection.

Initial Python CI exposed vendored metadata inside the setuptools wheel. The adapter now selects only top-level distribution metadata and checks its name/version against the wheel filename; focused tests also reject absent, duplicate and inconsistent identities. The first uv CI attempt exposed an unavailable Bookworm image tag; the Trixie tag above was verified against the upstream registry. The uv build invocation also disables its generated output `.gitignore`, keeping the packaging directory restricted to declared artifacts.

The initial Poetry path uses native Poetry lock freshness checks and the provisioned export plugin with project plugin loading disabled. The locked export supplies exact constraints and required hashes for wheel acquisition. Offline packaging invokes the project's captured PEP 517 backend (including the source-pinned `poetry-core` version), rather than substituting the frontend's own backend version. CI explicitly provisions the toolchain from `tooling/images/python-poetry.Dockerfile`; builds never install Poetry on demand. The added native case checks artifacts, captured backend identity, unittest results collected by pytest and rejection of a stale lock. Custom sources and non-PEP-621 project metadata remain outstanding.

The scenario harness can retain each generated bundle with `--evidence-dir`. CI uploads these as `oyzu-build-evidence`, including failed builds and their invocation exit codes. A summary is written only after all currently implemented checks pass; individual artifacts are evidence for their particular invocation, not an assertion that the entire scenario catalog passed.

[Run 36832368877](https://github.com/micahlmartin/oyzu/actions/runs/36832368877) passed at commit `2e83bd7dc8310b76900ba87b962790539c3ed59c` on 2026-10-01. All three host CLI and task jobs passed. The Linux native build job passed the existing Node/Go checks plus public Python acquisition, offline pip/uv/Poetry wheel and source-archive builds, native tests/reports, pip repeatability, uv/Poetry lock freshness rejection and captured dependency evidence. It uploaded the generated bundles. These are partial acceptance cases for the Python examples; legacy/native-extension, private-registry and remaining full-scenario requirements remain outstanding.

Inspecting that run's retained bundles found a coverage attribution defect that the initial checks did not catch: pip/uv report zero covered lines because coverage selected the checkout's package directory while tests imported the installed wheel. Poetry's default coverage also needs verification that it measures application code rather than only test code. The native tests passed, but correct application coverage remains incomplete. The Python adapter must select the installed distribution's sources and the scenario checks must assert coverage of executed application lines, not merely the presence of a coverage file or a positive total.

The correction now lives in Python's embedded reporting module. It derives importable modules and default report file selection from the installed distribution's native RECORD metadata, uses coverage.py `source_pkgs` to avoid directory-name ambiguity, and preserves native coverage settings through a build-private config file. The reporter still delegates execution, JUnit generation and threshold enforcement to pytest/pytest-cov. Local native regressions exercise a package and a single-file module with deliberately broken checkout shadows, plus a failing coverage threshold with passing tests. End-to-end CI now requires executed application coverage for pip, uv and Poetry and verifies that an unmet threshold blocks artifact packaging.

The first correction run (`36834433609`) failed in the reporter regression harness before native build scenarios: Python isolated mode could not import pytest installed in the Linux runner's user site. CI now provisions those tests in an explicit virtual environment, preserving isolated subprocess behavior.

[Run 36835174102](https://github.com/micahlmartin/oyzu/actions/runs/36835174102) passed at commit `d3841234890b78c7c481da4f63688c0ac28b7355`. Its downloaded bundles confirm application coverage of 8/10 lines for pip, 2/2 for uv and 2/2 for Poetry. The pip threshold case retained the 8/10 coverage and passing test results while failing the build and blocking artifacts. This establishes the coverage correction for these fixtures; it does not establish all native coverage configurations or Python packaging forms.

## Builder interfaces and module boundaries

Ecosystem-specific code is now owned by modules under `src/builders`, implementing a crate-private `Builder` trait. Discovery, toolchain selection, dependency preparation, typed planning and embedded runtime assets have explicit interface boundaries. The engine is split into lifecycle, planning, execution and bundle modules; it no longer selects native commands by package manager. Native report conversion remains in the shared report layer. See [builder code organization](builder-code-organization.md) for ownership and extension rules. This structural work preserves the existing supported behavior and does not expand the claimed builder acceptance scope.

Report evidence collection now has its own engine module, supporting OEP-0012's diagnostic bundle requirements. Contained reports up to 16 MiB are captured before parsing, so malformed native XML or invalid text remains inspectable with its digest while the action fails. Summaries are computed from the retained bytes. Focused tests cover invalid binary/text content, actual failed JUnit tests, unchanged captured evidence, and rejection of missing, oversized and escaping reports. This does not establish the full output-quota or filesystem race-resistance requirements.

## Checkpoint 5: initial Cargo workspace builds

[Run 36839115549](https://github.com/micahlmartin/oyzu/actions/runs/36839115549) passed at `ede93d0915843ae8d192bcc313f35b3637cd5955`. Downloaded bundles confirm native application/workspace/library crate archives, delivered binaries, repeated artifact digests, Clippy/rustfmt checks and actual application coverage (6/9, 13/18 and 10/12 lines respectively). Failed tests retained JUnit and coverage while blocking artifacts; the formatting failure also blocked packaging. This is evidence for the implemented local Cargo profiles, not registry acquisition, full feature/platform matrices or all Rust acceptance requirements.

The Rust adapter now implements preparation and planning through the same builder interface. Native `cargo metadata` owns workspace membership, and native locked/offline resolution checks the original lock before snapshot version projection. Only a private captured copy receives projected package versions and corresponding path-dependency requirements. Cargo regenerates and checks that copy's lock; a digest-bound manifest/lock overlay and typed workspace metadata become prepared inputs. Original checkout manifests and locks remain unchanged.

The initial profile builds local-only application workspaces with the explicitly provisioned `oyzu-toolchain/rust:1.94.0-nextest0.9.146` Linux toolchain image. Actions build release binaries, run native nextest with JUnit, run Clippy and perform nonmutating rustfmt checks. Build scripts and proc macros run inside the offline executor. Binary artifact names and native package snapshot versions are planned before execution; workspace artifacts can have independent versions. Each binary includes the compiler host triple in its filename. The CI toolchain Dockerfile currently provisions Linux x86_64 nextest with a verified archive checksum.

Rust owns its metadata validation, version projection and native commands under `src/builders/rust`. Common orchestration only gained general support for dynamic artifact names and per-artifact versions. Cargo discovery/development tasks remain native Cargo commands; build-mode test specialization supplies nextest reports. Native configuration comes from captured project files, not scenario expectation JSON.

The compiled-CLI scenario checks now exercise the Rust application and local workspace fixtures, repeated binary digests, delivery/execution of the binary, build-script input changes, proc macros, JUnit failures, source preservation and formatting failure. The fixtures were formatted to satisfy the real default formatting check. Local tests also cover inherited workspace versions, aliased path requirements and captured-path validation. Native CI verification of this increment is pending; neither Rust example is marked complete.

The first Cargo CI attempt exposed rejection of the underscore in `x86_64`; a new multi-artifact planning regression caught the same error and the validator was corrected. The following run (`36835557065`) passed all three host CLI/task jobs but failed earlier in Python acquisition: pip's 15-second timeout expired before the broker's 45-second upstream limit, creating overlapping retries. The adapter now allows 120 seconds per pip request with two retries; acquisition remains bounded by the executor. The harness runs network-free Cargo checks first and prints command progress/timing. This change still requires native verification.

Run `36836991932` reached a successful native Cargo build and nextest test, then caught an incorrect report lookup: nextest's default store is independent of `CARGO_TARGET_DIR`. The adapter now writes an absolute bundle JUnit destination into a private nextest configuration outside the source tree and passes it explicitly to nextest. A native Windows nextest probe verified that this destination works with a custom store; Rust regression tests preserve the project's store/retry/report settings and source configuration. The compiled-CLI Linux check is still authoritative for end-to-end acceptance.

Remaining Cargo requirements include registry/Git dependency acquisition, coverage and doctest reporting, feature/platform matrices, custom nextest store layouts and full Windows/macOS execution. External dependencies still fail explicitly. This increment does not fulfill the complete Rust builder contract in [OEP-0014](proposals/OEP-0014-builders-and-examples/implementation.md) or the full EX-021/022 acceptance cases.

### Cargo source archives

The adapter now declares an implicit `archive` task after the build/test/lint/format checks. It invokes native `cargo package` with locked offline resolution and package verification enabled, selecting every captured workspace package explicitly. Path dependencies acquire exact snapshot version requirements in the private overlay so Cargo can normalize them into distributable dependencies. Cargo owns its temporary local registry when verifying interdependent workspace archives; Oyzu does not synthesize crates or bypass native package verification.

Plans declare both executable binaries and per-package `.crate` files, including library-only projects. Artifact collection occurs only after every required check and native archive verification succeeds. Native Cargo packaging was exercised locally on the core/proc-macro/application workspace; the compiled-CLI CI cases additionally inspect archive metadata, dependency versions and contents, reject leaked build directories, compare repeated artifact digests and build a library-only project. End-to-end verification of this extension remains pending.

Run `36837775843` completed the first native application build, checks and packaging, then its schema validator rejected a dotted archive artifact identifier. Artifact names now use `bin-` and `crate-` prefixes, with a stable digest suffix when a native name must be shortened or normalized. Native package filenames remain intact. The shared planner validates artifact names, filename containment and case collisions before execution; it shares identifier rules with target configuration. This catches the contract defect locally rather than emitting an invalid successful bundle.

### Cargo coverage integration

The Rust toolchain image now provisions `cargo-llvm-cov` 0.9.1 using a pinned release archive checksum and installs the LLVM tools matching Rust 1.94.0. Its explicit image reference is `oyzu-toolchain/rust:1.94.0-nextest0.9.146-llvmcov0.9.1`. Preflight checks nextest, the coverage adapter and LLVM availability. `CARGO_LLVM_COV_SETUP=no` prohibits on-demand installation; all build/test actions remain offline.

The test action runs instrumented nextest, then generates native Cobertura while retaining the original test failure status. JUnit is written directly to its declared bundle destination. Separate coverage outputs live below the private Cargo target directory, excluded from source archives. The previous forced source-path remapping is removed: the executor already presents a stable `/workspace` path, and coverage must resolve actual captured source paths. Native project Rust flags are no longer shadowed by that remapping.

CI checks application-line coverage for the executable, workspace library and library-only case, including retention of coverage after failing tests. The workspace fixture's test now exercises its public function as well as its generated input. Coverage execution could not be verified with the local Windows GNU toolchain because that compiler lacks `profiler_builtins`; this is not counted as a passing coverage check. The Linux compiled-CLI scenario is pending. Doctest coverage, branch/feature/platform matrices and full native Windows/macOS build execution remain outstanding.

Run `36838292456` executed instrumented tests and generated native Cobertura, but report collection rejected its standard external DTD declaration. The shared parser now accepts only the known Cobertura declaration, without fetching it; internal subsets, entity declarations and other DTDs remain rejected. A regression checks the native header, foreign/external entities and a comment-hidden attempt to permit an internal subset. Report bytes remain unmodified and the parser has a node-count limit in addition to the existing file-size limit.


## Checkpoint 6: initial local Helm chart builds

The Helm adapter discovers a chart at the project root or in conventional `chart/` without build YAML. Ambiguous layouts require an explicit target path. The local profile traverses chart dependencies within the captured target, prepares nested dependencies before their consumers and invokes native `helm dependency build --skip-refresh` with networking disabled. Existing locks are validated by Helm; missing locks are generated only in the captured copy. Cycles, path escapes and remote sources fail explicitly pending the approved registry adapter.

The prepared chart closure includes a snapshot version on the primary chart, preserving application and dependency versions. Native Helm packages the chart, validates values/schema through rendering, and lints with subcharts. Only successful checks expose the chart archive and rendered YAML as bundle artifacts. There is no cluster deployment or fabricated JUnit/coverage report for template rendering, and no inferred source-mutating formatter. Explicit native format configuration remains outstanding.

Helm's native archive writer includes wall-clock timestamps. The ecosystem-owned archive adapter preserves entry bytes while normalizing transport metadata, without extracting files. Generated lock timestamps are normalized before the prepared snapshot; source-owned lock content is retained. Local native Helm 3.22.0 checks prove normalized archives reload, lint and render and repeat identically. Unit checks cover discovery, local dependency ordering, containment, cycles and planning. CI now exercises EX-028's actual source fixture, repeated plans/artifacts, source preservation, native stale-lock rejection and invalid values through the compiled CLI; that end-to-end verification is pending.

The image is explicitly provisioned by CI from the upstream Helm archive with its pinned checksum. Oyzu itself does not install tools. Remote/OCI chart acquisition, image artifact bindings, configurable Kubernetes capability matrices, chart test plugins and the rest of OEP-0018 remain required. This checkpoint does not mark the full Helm/container catalog complete.

Standalone library charts now retain native `type: library` semantics: lint and package them, without attempting to render an installable release or claiming application test results. The scenario harness builds the example's labels library independently and inspects its packaged helper templates. This also verifies root-level chart inference, complementing the nested application chart case.
