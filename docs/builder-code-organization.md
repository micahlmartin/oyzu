# Builder code organization

Apply the repository-wide [code organization rules](code-organization.md) alongside this ecosystem-specific guide.

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
      managers/empty.rs       # Initial dependency-free manager admission and evidence
      runtime/                # Native manager capture/replay, lifecycle and integrity validation
      reporting.rs            # Native test reporters and exact-command override adaptation
      jest.rs                 # Jest default invocation and exact script/override adaptation
      vitest.rs               # Vitest default invocation and exact script/override adaptation
    python/
      mod.rs                  # Descriptor and interface implementation
      discovery.rs            # pip / uv / Poetry inference
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
      runtime/metadata.go     # Native Go manifest/package inspection without running project code
      runtime/acquisition.go  # Native Go proxy protocol, checksum admission and module cache capture
    rust/
      mod.rs                  # Cargo descriptor, discovery and interface implementation
      metadata.rs             # Typed native workspace metadata and version projection
      preparation.rs          # Offline lock validation and captured manifest overlay
      planning.rs             # Binary, native checks and JUnit intent
      reporting.rs            # Private nextest settings and exact report destinations
      runtime/test.sh         # Native coverage/test reporting with failure preservation
      tests.rs                # Workspace projection and containment regressions
    java/{maven,gradle,ant}/   # Separate native-manager adapters
      maven/metadata.rs       # Typed native reactor metadata and output validation
      maven/preparation.rs    # Scoped native repository capture and POM overlay
      maven/planning.rs       # One native lifecycle with module artifact/report identities
      maven/runtime/          # Maven core metadata extension and native acquisition/lifecycle adapter
      gradle/metadata.rs      # Typed native composite models and path validation
      gradle/preparation.rs   # Scoped repository capture without mutable daemon caches
      gradle/planning.rs      # Native archive identities and module test evidence
      gradle/runtime/         # Native model, snapshot/check integration and acquisition transport
      ant/metadata.rs         # Typed native Ant output metadata and containment
      ant/preparation.rs      # Sandboxed native project evaluation
      ant/planning.rs         # Compile/check/archive intent and versioned JARs
      ant/reporting.rs        # Exact native test-target adaptation and required report contracts
      ant/runtime/            # Native Ant metadata and JDK archive integration
        AntTesting.java       # Native target execution and Java assertion outcomes
        AntJUnit.java         # Native JUnit task formatter/coverage integration
        AntReports.java       # Bounded native suite aggregation
        AntCoverage.java      # Application class ownership and JaCoCo reporting
    java/maven_repository.rs  # Shared Maven-layout inventory for Maven and Gradle
    docker/                   # Container builder
      metadata.rs             # Typed native facts and captured-input admission
      preparation.rs          # Offline native metadata and worker-profile capture
      planning.rs             # Snapshot OCI output and typed BuildKit action intent
      runtime/metadata/       # Pinned native BuildKit parser and Docker ignore facts
    helm/
      metadata.rs             # Chart discovery and contained local dependency order
      preparation.rs          # Native lock handling and captured chart closure
      planning.rs             # Packaging, linting and rendering commands
      runtime/archive.py      # Normalize native archive transport timestamps
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

`BuilderPlan.coverage` can declare typed application-source coverage applicability independently of report files. The shared planner serializes this fact into the target's `oyzu.dev/coverage-applicability` extension, which is retained in the manifest. Missing declarations do not mean inapplicable. Helm declares chart packaging inapplicable and produces native validation JUnit through its owned `runtime/testing.py` adapter; chart assertions are never converted to an application coverage percentage.

Command replacement and evidence requirements have separate ownership. A TOML override cannot remove the builder's required reports. The optional override adapter may instrument an exact known native command; the shared planner does not parse ecosystem commands or shell programs. Unknown replacements retain their arguments and receive `OYZU_TEST_REPORT` and `OYZU_COVERAGE_REPORT` destinations when those kinds have one concrete destination. Native stdout conversion applies only to native or recognized commands; arbitrary replacement output is not assumed to use the native event protocol. The shared reporting binder resolves custom declarations against captured task cwd and retains requirements for undeclared kinds. Hooks inherit the operation's report destinations. The Node adapter receives resolved destinations and owns conversion to reporter arguments.

Artifact names are owned strings, and artifacts can override the target-level version. This lets a native workspace expose multiple independently versioned package outputs without adding Cargo-specific cases to the engine. Cargo preparation uses the same captured-input interface as acquisition, recording native workspace metadata and a version-projected manifest/lock overlay; it does not require a separate execution path.

Native report intent can also identify a module and a contained file/glob under the task's working directory. Maven uses this to attribute Surefire and JaCoCo reports to reactor modules without adding Maven branches to the planner or collector. Shared report formats include native JaCoCo aggregate line counters. Prepared input snapshots include the entire acquired tree, even native coordinate directories named `target`; source checkout exclusion rules do not apply to repositories.

Gradle uses the same interface with its own composite metadata and native initialization scripts. Included builds export their own evaluated models. Preparation captures repository files through the shared broker; the offline lifecycle resolves a local file repository with a fresh private Gradle home. Mutable daemon/dependency caches are not prepared inputs. Maven and Gradle share the Maven-layout file inventory. Node, Maven and Gradle share the Rust preparation lifecycle in `dependencies/preparation.rs`. That lifecycle owns temporary workspace cleanup, scoped broker lifetime, sandbox invocation and prepared-tree capture. Each manager supplies explicit commands, runtime assets, environment and source routes; native acquisition semantics, metadata validation and planning stay in its own adapter. No manager switch is needed in the shared lifecycle.

Node captures lockfile-addressed npm registry tarballs and a deterministic inventory. Its adapter uses native `npm cache add` with a fresh temporary cache, then native `npm ci --offline` to validate and install; mutable npm cache indexes are not frozen inputs. Acquisition disables lifecycle scripts while the broker is mounted. Build execution seeds another private cache from the captured tarballs and runs native installation with lifecycle scripts enabled inside the network-isolated executor, without a broker mount. All source lockfiles remain untouched; snapshot version projection changes only the private execution copy.

Go preparation uses a private native module cache and a loopback GOPROXY adapter that forwards requests through the engine's scoped spool. `broker/runtime/transport.go` owns the Go spool protocol client; the Go builder owns module URLs, native checksums and inventory. Native manifests/checksums must remain unchanged. Preparation captures the whole module tree and archive identities; actions mount it read-only with module downloads and VCS fetching disabled. Public-proxy acquisition is the initial source profile, with managed/private connectors still pending.

The executor owns process invocation and sandbox flags. A builder's planned command does not grant a host mount, credentials or network access. The broker owns request validation and upstream authorization; each acquisition adapter supplies its configured source routes. Dependency-free adapters return no prepared snapshot. Discovery-only adapters return an explicit error for unimplemented build behavior.

Native runtime code belongs to its ecosystem. The embedded Python adapter exists to invoke native package tooling and inspect native metadata inside the isolated toolchain environment. It does not own scheduling, policy decisions or bundle finalization. Further Python growth should split manager and operation modules inside `builders/python`, not add unrelated ecosystems to a global helpers directory.

Python requirements-only application planning lives in `application.rs`, with archive assembly and artifact-source test execution in `runtime/application.py`. Package/application plans reuse `quality.rs` for native Ruff tasks, the acquisition adapter for prepared environments, and `runtime/reporting.py` for native pytest/coverage integration. The prepared dependency record's `oyzu.dev/python-runtime` extension identifies runtime roots; application packaging consumes their captured closure. Distribution metadata is never synthesized to fit a requirements application into the package builder. Archive collection, required reports and action ordering remain shared engine responsibilities.

Python quality selection lives in `detection/quality.rs`: independent linter and formatter detectors produce evidence for the common resolver, with Ruff defaults and native Black/Flake8 configuration support. `quality.rs` declares development commands and captured execution intent. The acquisition adapter adds only selected missing tools, respecting declared/native-lock versions. `runtime/quality.py` owns native quality invocation and exclusion of private engine state while preserving project exclusions. These details do not introduce Python tool switches into orchestration. Ambiguous explicit selections fail discovery; unsupported frameworks remain extension work.

Legacy setuptools metadata has a separate boundary in `python/legacy.rs` and `runtime/legacy.py`. Static discovery never evaluates setup scripts. Acquisition closes its broker before the legacy adapter asks the shared executor to evaluate native metadata in a private workspace with read-only dependencies and no broker mount. Typed native identity/compiler facts are retained in the dependency snapshot and consumed by the common Python package planner. Native snapshot tagging remains setuptools-owned, while report collection, sandboxing and artifact identity enforcement retain their shared owners.

Report collection is a separate engine responsibility. It retains contained raw report bytes before parsing, within the shared 16 MiB report limit. Parsing and digests refer to those retained bytes. Invalid reports fail the action but remain available for diagnosis; missing, escaping or oversized files are not copied into the bundle. Native report formats remain in `reports.rs`, rather than being implemented separately by each builder.

The collector queues executed producers until their post-hook boundary. It also drains a failed producer when that post-hook is blocked, and retains available evidence when post itself fails. Declared globs use the same typed formats and bounded capture as native reports; every matched file receives a stable identity and independent validation. Glob expansion does not follow symlinks. The scheduler owns outcome changes and downstream blocking; the collector does not run commands or choose build stages.

Materialization is a shared graph responsibility. Its planner resolves logical artifact outputs and contained consumer destinations before bytes exist; its executor copies only successfully produced, digest-verified bundle files. Each target has a private mutable source workspace, and collectors read that target's report paths. Consumers never mount producer bundle storage. The initial implementation handles file artifacts on matching OS/architecture pairs; directory outputs and richer runtime compatibility still require integration.

Builders can declare an optional source-file selection in `BuilderPlan`. The snapshot module binds that selection to the target path, retains required parent directories and leaves other workspace targets available. The planner records the selection in the target's `oyzu.dev/source-projection` extension and checks materialization collisions against the selected source. Execution uses that same selection to create the private consumer workspace before materializing artifacts. Docker owns the native ignore interpretation and reserves its Dockerfile/ignore control files; the engine needs no Docker-specific branch. Ignored source files can therefore be replaced by explicit artifact inputs without changing the checkout or weakening collision checks for included files.

## Adding a builder

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
