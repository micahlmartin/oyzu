# Builder code organization

Apply the repository-wide [code organization rules](code-organization.md) alongside this ecosystem-specific guide.

Dockerfile-free packaging uses a shared [container assembly boundary](container-assembly.md). `Builder::container_profile` supplies language-owned runtime/artifact requirements; `build/containers` composes acquisition and derived packaging actions, and the executor owns the bounded generated definition and isolated worker. Do not introduce per-language Dockerfile writers or recompile the application during packaging. The first Python profile is verified in Linux native acceptance; general profiles remain unfinished.

Provisioned base-image capture and its native converter live under `dependencies/images` and `dependencies/runtime/images`. Docker owns parsing image requirements from Dockerfiles and consumes this shared acquisition contract. Application builders must use the same contract after configuration admission, while retaining their own language-runtime compatibility checks.

`PreparationContext.configuration` is a required reference to the owner's resolved snapshot. Consume registered settings from that snapshot before effects; do not reread TOML/environment inputs or recompute setting defaults. Missing build snapshots fail at the composition boundary, including inferred projects that have no configuration files. Defaults are established once by configuration resolution.

Builders may register native offline-store providers through `dependency_providers`. The consumer is shared `dependencies/context` composition; Docker does not import Python internals. Providers define a native manifest match, required tools, preparation operation and relative store directory. Python's closed `PythonStore` variants cover pip, uv and Poetry, reusing wheel resolution and inventory while excluding build/test/quality roots. Locked profiles delegate runtime-group selection to native exporters, and the runtime emits a hashed install manifest from the resolved wheels. Unsupported ecosystems and multiple native managers still participate in ambiguity detection; explicit selection never bypasses policy or runtime checks. This is an internal contract, not a public plugin ABI or a promise of all-manager integration.

This describes the current Rust implementation structure. The behavioral design remains in [OEP-0014](proposals/OEP-0014-builders-and-examples/implementation.md), with acquisition in [OEP-0017](proposals/OEP-0017-dependency-acquisition/README.md). This refactor does not make unimplemented builder profiles complete; see [implementation status](implementation-status.md).

## Ownership

```text
src/
  builders/
    mod.rs                    # Built-in registration and lookup
    contract.rs               # Builder trait and typed planning contracts
    task.rs                   # Shared implicit-task constructors
    node/
      mod.rs                  # Descriptor and interface implementation
      discovery.rs            # Tasks from resolved manager/framework facts
      detection/managers.rs   # Native manager declarations and lock evidence
      detection/frameworks.rs # Test script/config/dependency observations and fallback
      planning.rs             # Shared Node version, artifact and report intent
      managers/mod.rs         # Crate-private native Manager interface and registration
      managers/{npm,pnpm,yarn}.rs # Native toolchain, preparation, script and packaging behavior
      managers/npm/preparation.rs # npm lock admission and scoped tarball snapshot records
      managers/registry.rs    # Shared immutable-archive snapshot evidence for pnpm/Yarn
      managers/pnpm/patches.rs # Source-patch evidence after native frozen validation
      runtime/                # Native manager capture/replay, lifecycle and integrity validation
      reporting.rs            # Native test reporters and exact-command override adaptation
      workspace.rs            # Shared package test scopes and member admission
      workspace/testing.rs    # Native observation transport and frozen host report plans
      workspace/reporting.rs  # Framework selection and named JUnit/LCOV obligations
      runtime/workspace-testing.mjs # Shared package test execution; manager supplies script command
      runtime/{pnpm,yarn}-workspace-describe.mjs # Manager-owned native membership observers
      runtime/native-workspace.mjs # Provisioned entrypoints and bounded observation transport
      jest.rs                 # Jest default invocation and exact script/override adaptation
      vitest.rs               # Vitest default invocation and exact script/override adaptation
    python/
      mod.rs                  # Descriptor and interface implementation
      discovery.rs            # pip / uv / Poetry inference
      detection/testing.rs    # Native pytest evidence and profile fallback
      acquisition.rs          # Native locks and scoped wheel acquisition
      planning.rs             # Python commands and artifact/report intent
      runtime/adapter.py      # Embedded native Python adapter
      runtime/reporting.py    # Installed-distribution pytest/coverage integration
    go/
      mod.rs                  # Go descriptor and static implicit tasks
      development.rs          # Native workspace arguments/environment only on explicit task run
      metadata.rs             # Typed portable native facts and validation
      preparation.rs          # Shared isolated capture, toolchain and compiler evidence
      planning.rs             # Workspace checks and named snapshot binary artifacts
      packaging.rs            # Native module artifact record admission
      runtime/metadata.go     # Native Go manifest/package inspection without running project code
      runtime/acquisition.go  # Native Go proxy protocol, checksum admission and module cache capture
      runtime/modulezip/      # Pinned x/mod integration for source archive projection and checksums
    rust/
      mod.rs                  # Cargo descriptor, discovery and interface implementation
      acquisition.rs          # Locked crates.io archive/index capture through scoped transport
      metadata.rs             # Native workspace/features, binary selection and version projection
      preparation.rs          # Offline lock validation, manifest overlay and binary inventory
      planning.rs             # Snapshot artifacts, fixed target facts, native checks and reports
      packaging.rs            # Native path-dependency ordering for mixed registry workspaces
      reporting.rs            # Private nextest settings and exact report destinations
      testing.rs              # Shared captured/host test selection and report obligations
      runtime/test.py         # Cross-platform nextest, coverage and doctest failure preservation
      runtime/doctest.py      # Stable Cargo invocations with explicitly scoped JUnit
      runtime/build.py        # Native compiler-message binding and contained executable staging
      runtime/package.py      # Verified native archives/indexes in a private registry overlay
      tests.rs                # Workspace projection and containment regressions
    java/quality.rs           # Shared native Java quality task defaults and invocation mapping
    java/runtime/             # Owned Java source launcher and default Checkstyle rules
    java/{maven,gradle,ant}/   # Separate native-manager adapters
      maven/metadata.rs       # Typed native reactor metadata and output validation
      maven/preparation.rs    # Scoped native repository capture and POM overlay
      maven/planning.rs       # One native lifecycle with module artifact/report identities
      maven/testing.rs        # Admitted native model observation and direct module report contracts
      maven/runtime/          # Maven core metadata extension and native acquisition/lifecycle adapter
      maven/runtime/reporting.py # Frozen native report-directory capture; shared collection owns parsing
      maven/runtime/host.py   # Provisioned host model/test commands, no snapshot version projection
      gradle/metadata.rs      # Typed native composite models and path validation
      gradle/preparation.rs   # Scoped repository capture without mutable daemon caches
      gradle/planning.rs      # Native archive identities and module test evidence
      gradle/testing.rs       # Shared native test admission and direct composite report plan
      gradle/runtime/         # Native model, snapshot/check integration and acquisition transport
      gradle/runtime/reporting.gradle # Shared native JUnit/JaCoCo integration
      gradle/runtime/host.py  # Offline model/test commands with native versions and repositories
      gradle/runtime/host.gradle # Frozen test selection across root and included builds
      ant/metadata.rs         # Typed native Ant output metadata and containment
      ant/preparation.rs      # Sandboxed native project evaluation
      ant/planning.rs         # Compile/check/archive intent and versioned JARs
      ant/reporting.rs        # Exact native test-target adaptation and required report contracts
      ant/runtime/            # Native Ant metadata and JDK archive integration
        testing.py            # Shared captured/host reporting launcher and native tool paths
        AntTesting.java       # Native target execution and Java assertion outcomes
        AntJUnit.java         # Native JUnit task formatter/coverage integration
        AntReports.java       # Bounded native suite aggregation
        AntCoverage.java      # Application class ownership and JaCoCo reporting
    java/maven_repository.rs  # Shared Maven-layout inventory for Maven and Gradle
    java/reporting.rs         # Shared native JUnit composition runtime registration
    java/runtime/junit.py     # Compose native suites; collection owns validation/counts
    docker/                   # Container builder
      quality.rs              # Native linter/formatter detector evidence and task defaults
      metadata.rs             # Typed native facts and captured-input admission
      preparation.rs          # Offline native metadata and worker-profile capture
      planning.rs             # Snapshot OCI output and typed BuildKit action intent
      runtime/metadata/       # Pinned native BuildKit parser and Docker ignore facts
    helm/
      quality.rs              # Implicit native YAML formatter tasks and host command adaptation
      runtime/quality.py      # Bounded chart YAML selection and native read-only/mutating formatting
      archives.rs             # Bounded static archive-member observations; no extraction
      runtime/charts.py       # Private chart expansion and native assertion execution
      detection.rs            # Bounded root/unpacked-subchart suite evidence and validation fallback
      metadata.rs             # Chart discovery and contained local dependency order
      preparation.rs          # Native lock handling and captured chart closure
      planning.rs             # Packaging, linting and rendering commands
      runtime/archive.py      # Normalize native archive transport timestamps
      runtime/testing.py      # Native validation/unittest reports and baseline integrity
  build/
    mod.rs                    # Capture, preparation, execution and finalization lifecycle
    planning.rs               # Common hook expansion and execution-plan serialization
    execution.rs              # Scheduling, executor invocation and outcome collection
    materialization.rs        # Symbolic artifact selection and verified private consumer copies
    collection.rs             # Bounded report capture, parsing and failure evidence
    reporting.rs              # Required evidence and custom report location bindings
    bundle.rs                 # Output containment, capture and integrity inspection
  discovery.rs                # Workspace ownership, ambiguity and task overrides
  dependencies.rs             # Shared prepared dependency snapshot
  dependencies/preparation.rs # Temporary capture workspace, scoped broker and sandbox lifecycle
  broker.rs                   # Source-scoped transport and credential boundary
  broker/runtime/             # Credential-free acquisition clients for Python and Node
  executor.rs                 # Image admission and ordinary process execution
  executor/mode.rs            # Typed process/BuildKit capabilities
  executor/worker.rs          # Private rootless worker lifecycle and filtered context assembly
  oci/                        # Bounded layout/archive and descriptor integrity verification
  reports.rs                  # Native report conversion and validation
  reports/contract.rs         # Typed declarations and bounded contained glob discovery
```

Modules are private unless a public CLI/library entry point needs them. The `Builder` trait and planning types are crate-private; they are not an external plugin ABI. A single crate is sufficient at this stage. A future crate split should follow an actual reuse or isolation boundary, rather than requiring separate crates for small adapters.

## Builder contract

| Method | Responsibility |
| --- | --- |
| `descriptor` | Declare the builder IDs the adapter owns |
| `detect` | Identify conventional native manifests without executing code |
| `discover` | Read native metadata and declare implicit development tasks |
| `development_command` | Resolve typed native arguments/environment during explicit development execution; never during static detection |
| `executor_profile` | Select a finite engine-owned toolchain capability profile |
| `toolchain` | Select the provisioned toolchain for the detected manager, or report unsupported integration |
| `prepare` | Capture native dependencies through the scoped broker and executor; return an immutable-input record |
| `plan` | Return typed command, task, artifact and report intent using captured source and prepared inputs |
| `instrument_override` | Optionally add native reporting to an exactly recognized replacement command; required evidence stays owned by the operation |
| `runtime_files` | Declare compiled-in adapter assets needed by isolated native processes |

`BuilderPlan`, `CommandSpec`, `TaskPlan`, `ArtifactSpec` and `ReportSpec` are Rust structures. A builder does not assemble arbitrary build-plan JSON. The common planner expands hooks, preserves TOML replacements, assigns action identities, binds source/dependency/toolchain identities and serializes the versioned plan. Report formats and input conversions are explicit types.

`BuilderPlan.coverage` and `TaskPlan.coverage` can declare typed application-source coverage applicability independently of report files for captured builds and direct host tests, respectively. The shared planner serializes this fact into the target's `oyzu.dev/coverage-applicability` extension, which is retained in the manifest. Missing declarations do not mean inapplicable. Helm declares chart packaging inapplicable and produces native validation JUnit through its owned `runtime/testing.py` adapter; chart assertions are never converted to an application coverage percentage. Helm's private `detection.rs` implements conventional native-suite evidence and validation fallback through the common detector resolver. Helm `testing.rs` owns the shared validation/unittest commands and report obligations; both planning and `development_test` use it. Planning adds a separate required unittest JUnit report when selected. The host adapter copies the chart through the existing chart-copy boundary so native Helm never reads the live Oyzu lease or old bundles. Static archives use the shared bounded binary source capture; Helm owns member/path interpretation. The owned runtime expands chart archives only in private trees, requires native test cases and checks snapshot baseline integrity; the shared engine still owns hooks, scheduling and report collection. See the [Helm reference](reference/helm.md).

Command replacement and evidence requirements have separate ownership. A TOML override cannot remove the builder's required reports. The optional override adapter may instrument an exact known native command; the shared planner does not parse ecosystem commands or shell programs. Unknown replacements retain their arguments and receive `OYZU_TEST_REPORT` and `OYZU_COVERAGE_REPORT` destinations when those kinds have one concrete destination. Native stdout conversion applies only to native or recognized commands; arbitrary replacement output is not assumed to use the native event protocol. The shared reporting binder resolves custom declarations against captured task cwd and retains requirements for undeclared kinds. Hooks inherit the operation's report destinations. The Node adapter receives resolved destinations and owns conversion to reporter arguments.

Artifact names are owned strings, and artifacts can override the target-level version. This lets a native workspace expose multiple independently versioned package outputs without adding Cargo-specific cases to the engine. Cargo preparation uses the same captured-input interface as acquisition, recording native workspace metadata and a version-projected manifest/lock overlay; it does not require a separate execution path.

Native report intent can also identify a module and a contained file/glob under the task's working directory. Maven uses this to attribute Surefire and JaCoCo reports to reactor modules without adding Maven branches to the planner or collector. Shared report formats include native JaCoCo aggregate line counters. Prepared input snapshots include the entire acquired tree, even native coordinate directories named `target`; source checkout exclusion rules do not apply to repositories.

Gradle uses the same interface with its own composite metadata and native initialization scripts. Included builds export their own evaluated models. Preparation captures repository files through the shared broker; the offline lifecycle resolves a local file repository with a fresh private Gradle home. Mutable daemon/dependency caches are not prepared inputs. Maven and Gradle share the Maven-layout file inventory. Node, Maven and Gradle share the Rust preparation lifecycle in `dependencies/preparation.rs`. That lifecycle owns temporary workspace cleanup, scoped broker lifetime, sandbox invocation and prepared-tree capture. Each manager supplies explicit commands, runtime assets, environment and source routes; native acquisition semantics, metadata validation and planning stay in its own adapter. No manager switch is needed in the shared lifecycle.

Node captures lockfile-addressed npm registry tarballs and a deterministic inventory. Its adapter uses native `npm cache add` with a fresh temporary cache, then native `npm ci --offline` to validate and install; mutable npm cache indexes are not frozen inputs. Acquisition disables lifecycle scripts while the broker is mounted. Build execution seeds another private cache from the captured tarballs and runs native installation with lifecycle scripts enabled inside the network-isolated executor, without a broker mount. All source lockfiles remain untouched; snapshot version projection changes only the private execution copy.

Native npm workspace membership and local dependency edges are captured by `runtime/npm-workspaces.mjs`, using the provisioned npm's map-workspaces and Arborist libraries. `npm-native.mjs` binds commands and native libraries to the same installed entrypoint. `managers/npm/workspace.rs` validates the resulting typed records before they enter the dependency snapshot. Workspace manifests remain source inputs; registry tarballs remain acquisition inputs. Workspace task/artifact planning is a separate remaining responsibility and must consume these facts rather than manufacture a root-only artifact or infer npm glob/edge semantics in shared orchestration.

Go preparation uses a private native module cache and a loopback GOPROXY adapter that forwards requests through the engine's scoped spool. `broker/runtime/transport.go` owns the Go spool protocol client; the Go builder owns module URLs, native checksums and inventory. Native manifests/checksums must remain unchanged. Preparation captures the whole module tree and archive identities; actions mount it read-only with module downloads and VCS fetching disabled. Public-proxy acquisition is the initial source profile, with managed/private connectors still pending.

The executor owns process invocation and sandbox flags. A builder's planned command does not grant a host mount, credentials or network access. The broker owns request validation and upstream authorization; each acquisition adapter supplies its configured source routes. Dependency-free adapters return no prepared snapshot. Discovery-only adapters return an explicit error for unimplemented build behavior.

Native runtime code belongs to its ecosystem. The embedded Python adapter exists to invoke native package tooling and inspect native metadata inside the isolated toolchain environment. It does not own scheduling, policy decisions or bundle finalization. Further Python growth should split manager and operation modules inside `builders/python`, not add unrelated ecosystems to a global helpers directory.

Python test discovery records native pytest configuration or the profile fallback through `detection/testing.rs`. It does not guess collection results from directory names. The native runner owns actual selection and no-tests outcomes; acquisition prepares reporter inputs independently of directory layout. Report parsing and action/artifact failure propagation remain shared engine responsibilities. The [Python testing reference](reference/python-testing.md) distinguishes direct task behavior from captured-build evidence.

Python requirements-only application planning lives in `application.rs`, with archive assembly and artifact-source test execution in `runtime/application.py`. Package/application plans reuse `quality.rs` for native Ruff tasks, the acquisition adapter for prepared environments, and `runtime/reporting.py` for native pytest/coverage integration. The prepared dependency record's `oyzu.dev/python-runtime` extension identifies runtime roots; application packaging consumes their captured closure. Distribution metadata is never synthesized to fit a requirements application into the package builder. Archive collection, required reports and action ordering remain shared engine responsibilities.

Distribution application intent lives in `python/distribution_app.rs`, extending the native package plan with one declared console entrypoint and an application artifact. Its runtime assembles the backend-produced wheel plus captured runtime inputs, while `runtime/application.py` owns shared archive writing, extracted-source testing and digest checks. Package metadata stays native-owned; no second backend build, checkout-copy packaging or engine-level application special case is introduced. See the [application reference](reference/python-applications.md) for format limits.

Python quality selection lives in `detection/quality.rs`: independent linter and formatter detectors produce evidence for the common resolver, with Ruff defaults and native Black/Flake8 configuration support. `quality.rs` declares development commands and captured execution intent. The acquisition adapter adds only selected missing tools, respecting declared/native-lock versions. `runtime/quality.py` owns native quality invocation and exclusion of private engine state while preserving project exclusions. These details do not introduce Python tool switches into orchestration. Ambiguous explicit selections fail discovery; unsupported frameworks remain extension work.

Legacy setuptools metadata has a separate boundary in `python/legacy.rs` and `runtime/legacy.py`. Static discovery never evaluates setup scripts. Acquisition closes its broker before the legacy adapter asks the shared executor to evaluate native metadata in a private workspace with read-only dependencies and no broker mount. Typed native identity/compiler facts are retained in the dependency snapshot and consumed by the common Python package planner. Native snapshot tagging remains setuptools-owned, while report collection, sandboxing and artifact identity enforcement retain their shared owners.

Report collection is a separate engine responsibility. It retains contained raw report bytes before parsing, within the shared 16 MiB report limit. Parsing and digests refer to those retained bytes. Invalid reports fail the action but remain available for diagnosis; missing, escaping or oversized files are not copied into the bundle. Native report formats remain in `reports.rs`, rather than being implemented separately by each builder.

The collector queues executed producers until their post-hook boundary. It also drains a failed producer when that post-hook is blocked, and retains available evidence when post itself fails. Declared globs use the same typed formats and bounded capture as native reports; every matched file receives a stable identity and independent validation. Glob expansion does not follow symlinks. The scheduler owns outcome changes and downstream blocking; the collector does not run commands or choose build stages.

Materialization is a shared graph responsibility. Its planner resolves logical artifact outputs and contained consumer destinations before bytes exist; its executor copies only successfully produced, digest-verified bundle files. Each target has a private mutable source workspace, and collectors read that target's report paths. Consumers never mount producer bundle storage. The initial implementation handles file artifacts on matching OS/architecture pairs; directory outputs and richer runtime compatibility still require integration.

Builders can declare an optional source-file selection in `BuilderPlan`. The snapshot module binds that selection to the target path, retains required parent directories and leaves other workspace targets available. The planner records the selection in the target's `oyzu.dev/source-projection` extension and checks materialization collisions against the selected source. Execution uses that same selection to create the private consumer workspace before materializing artifacts. Docker owns the native ignore interpretation and reserves its Dockerfile/ignore control files; the engine needs no Docker-specific branch. Ignored source files can therefore be replaced by explicit artifact inputs without changing the checkout or weakening collision checks for included files.

Docker's `images.rs` prepares literal image inputs using the executor's provisioned-image export operation and the native Go image adapter. The adapter preserves/validates config and layer content in an OCI store; `ImageInput` supplies relative store bindings and frozen identities to the private worker. The worker owns containment, digest-checked private copies and native OCI context mounts. No image-format parser, registry client or Dockerfile-name switch is added to shared scheduling. See [image-input behavior](reference/docker-images.md) for the current provisioning profile and deferred registry acquisition.

## Adding a builder

The npm workspace planner lives under `node/managers/npm/workspace/`. It consumes captured native membership/edges and declares per-member snapshot artifacts and required report paths through the existing builder contract. Its owned runtime projects versions into the private execution copy, invokes native npm scripts/packing, and preserves package digests between build and collection. Shared planning still owns task overrides/hooks and required evidence. Native root scripts take precedence over member fan-out; absent root scripts use captured dependency order. Workspace discovery does not execute npm to populate development tasks.

Publishable workspace roots declare an additional package and root report obligations. `node/runtime/npm-workspace-root.mjs` owns native root pack selection/staging and implicit root test scopes. It filters engine-owned state from npm's selected files, then asks npm to create the archive and verifies that file selection stayed unchanged. Node defaults exclude member trees; Jest/Vitest receive native scope filters that preserve their configured exclusions. Explicit root test scripts retain their aggregate semantics. Root packaging and test scope do not add ecosystem-specific branches to shared scheduling or collection.

The Node manager contract also has an optional development-command resolver, called only for explicit `oyzu run` execution. npm's workspace resolver reads installed native membership/edges through its existing adapter, validates the typed metadata and reuses the captured planner's dependency ordering. It returns one invocation to the task engine; an owned launcher invokes member scripts sequentially with native npm pre/post lifecycle behavior. Static task listing requires no Node/npm process. Explicit root scripts and user overrides bypass this implicit aggregate resolver. Development execution does not project snapshot versions or manufacture dist evidence.

Workspace quality composition uses the same native quality runtime for unscripted members. Root default scans exclude member roots; each member default scan excludes its nested members. These contained scopes come from the captured membership model, not another user configuration language. A root quality script owns the whole operation once; otherwise each member's native script or default runs. Formatting aliases map to one workspace stage, while the task engine continues to own stage hooks and explicit task replacement.

`node/managers/npm/workspace/scope.rs` computes those ownership scopes for both captured plans and development plans. The development quality composer consumes the existing Node detector profiles and emits typed native-script or default-checker operations. The launcher reuses the owned ESLint/Prettier runtime, attempts each package after a quality failure, and returns failure for the aggregate. Explicit `format` uses the same scopes but is marked mutating and is not an implicit build stage. Default quality argument overrides are rejected before execution; custom native scripts and argv tasks provide the escape hatch.

The development test composer similarly uses the existing framework detectors and emits native-script or implicit-test operations. `workspace-test-scope.mjs` owns native entrypoint resolution and package test selection, shared by development and captured adapters. It is separate from native root packaging. Captured member scopes are explicit plan facts; member scripts retain native selection and lifecycle semantics. This does not move task hooks, scheduling or report collection into the adapter, and development tests do not claim captured report evidence.

Node quality discovery has separate linter/formatter detectors under `node/detection/quality.rs`. `node/quality.rs` supplies implicit tasks and binds their native runtime for builds and explicit development execution. The runtime uses ESLint/Prettier APIs, preferring installed project tools and otherwise explicitly provisioned image defaults. It owns native configuration/rule handling and source selection; shared orchestration owns task ordering, overrides and failure propagation. The image's lockfile is generated by npm, not reconstructed by Oyzu.

Biome uses that same selection/ownership path and invokes its installed native CLI through its npm entrypoint. The quality runtime owns bounded file-argument batches and preserves any failed batch. Check versus explicit write is encoded by the owned command, not a user-forwarded flag. Single-package and workspace defaults reject extra arguments before checker execution; native scripts remain the explicit customization boundary. Tool-specific rule/configuration semantics stay with native libraries and CLIs.

Every builder must implement the [default testing contract](proposals/OEP-0014-builders-and-examples/testing.md): implicit test discovery, existing-framework selection or documented fallback, JUnit and applicable coverage intent. Native detection/instrumentation lives in the ecosystem module; the engine owns required evidence and dist collection. A task override cannot erase these obligations, and an unimplemented reporter remains an explicit implementation gap. Packaging builders declare applicability and exact producer evidence relationships rather than inventing source coverage.

Implement the trait in the ecosystem module and register it once in `builders/mod.rs`. Put native inference and version projection in that adapter, reuse task constructors and shared report formats, and declare exact output identities before execution. Add native scenario verification for its artifacts and failure behavior. Do not add a new manager switch to the shared engine, let native acquisition contact arbitrary sources, or manufacture successful results for missing integrations.

Tests cover unique registrations, native ownership ambiguity, required prepared inputs, typed Python output/report intent and preservation of explicit task overrides. Existing discovery, hook, bundle and native CI scenario tests remain the behavior checks across this structural change. Cross-platform runtime support must still be demonstrated independently.

## Encapsulation as builders grow

Discovery is moving from coarse `Builder::detect` and mutable per-builder inference to [specialized detectors plus a resolver](proposals/OEP-0006-discovery-and-planning/detectors.md). Shared types and exclusive-role resolution now live in `src/discovery/detectors.rs`, with bounded read-once metadata in `source.rs`. Node and Python manager detectors and Node test-framework detectors live under their ecosystem's `detection/` modules. They report typed evidence without creating tasks; task discovery consumes the selected manager. Target plan/manifest extensions retain registry versions, source evidence and observations. Coarse builder ownership, remaining framework/suite composition and deferred native metadata still require migration. No public plugin ABI or user priority/configuration DSL is introduced.

Use private child modules by default and the narrowest useful visibility (`pub(super)` or `pub(in crate::builders)`) for implementation seams. The engine depends on the `Builder` contract, never a concrete manager's metadata or runtime modules. Keep native representations inside their ecosystem and convert them to shared intent at that boundary.

Extract shared behavior when multiple implementations have the same responsibility and invariants. Place it at their nearest common owner: Maven repository operations belong under Java, while scheduling, integrity verification and sandbox enforcement belong to the shared engine. Similar-looking native commands alone do not justify merging adapters. Prefer concrete types and small functions; introduce another trait only when there are distinct implementations or an actual substitution boundary.

Keep this as one crate while module privacy provides the needed separation. Separate crates become useful when a component has independent reuse, dependency or release requirements. A future external plugin interface needs its own versioning and isolation design; the internal Rust trait does not promise that interface.


Container actions use a typed executor mode declared by the Docker builder. The shared planner binds its exact BuildKit argv and capability description into the action; the scheduler dispatches by that mode, not by ecosystem name. Normal hooks and explicit command overrides remain ordinary process actions. The worker owns context mounts, rootless startup, readiness checks, timeout/log enforcement and confirmed shutdown before copying outputs. Builders cannot add arbitrary host mounts or privileged entitlements through this interface.

`OciValidation` is a separate engine-owned mode: it reads a contained completed OCI archive, invokes the shared bounded verifier, checks the planned platform, and writes JUnit through the shared assertion encoder. It executes no project command and does not claim a container ran. `executor/files.rs` owns contained input/exclusive output operations shared with the worker. Builder-provided native operations can satisfy build-time availability even when direct development execution is unavailable; shared task traversal still owns ordering, hooks and cycle detection, while host execution retains its availability checks.

Materialization receipts link the original producer's collected test/coverage records, source subjects and digests alongside the exact copied artifact digest. The materialization owner checks report bytes and successful producer actions before recording these references. References explicitly have producer-target scope; consumers do not receive copied reports or claim they measured the producer's coverage again.

Artifact kind is also typed (`File` or `OciImage` in the current adapters). Shared collection verifies OCI structure/content before recording an image's publication digest; the Docker adapter does not manufacture manifest records. The OCI module is shared integrity infrastructure, separate from Dockerfile semantics and the BuildKit process lifecycle.
