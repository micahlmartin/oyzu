# Builder code organization

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
      discovery.rs            # Native metadata, managers and scripts
      planning.rs             # npm commands and artifact/report intent
      reporting.rs            # Native test reporters and exact-command override adaptation
    python/
      mod.rs                  # Descriptor and interface implementation
      discovery.rs            # pip / uv / Poetry inference
      acquisition.rs          # Native locks and scoped wheel acquisition
      planning.rs             # Python commands and artifact/report intent
      runtime/adapter.py      # Embedded native Python adapter
      runtime/reporting.py    # Installed-distribution pytest/coverage integration
    go/                       # Discovery and Go planning
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
      ant/metadata.rs         # Typed native Ant output metadata and containment
      ant/preparation.rs      # Sandboxed native project evaluation
      ant/planning.rs         # Compile/check/archive intent and versioned JARs
      ant/runtime/            # Native Ant metadata and JDK archive integration
    docker/                   # Container builder
    helm/
      metadata.rs             # Chart discovery and contained local dependency order
      preparation.rs          # Native lock handling and captured chart closure
      planning.rs             # Packaging, linting and rendering commands
      runtime/archive.py      # Normalize native archive transport timestamps
  build/
    mod.rs                    # Capture, preparation, execution and finalization lifecycle
    planning.rs               # Common hook expansion and execution-plan serialization
    execution.rs              # Scheduling, executor invocation and outcome collection
    collection.rs             # Bounded report capture, parsing and failure evidence
    reporting.rs              # Required evidence and custom report location bindings
    bundle.rs                 # Output containment, capture and integrity inspection
  discovery.rs                # Workspace ownership, ambiguity and task overrides
  dependencies.rs             # Shared prepared dependency snapshot
  broker.rs                   # Source-scoped transport and credential boundary
  broker/runtime/transport.py # Shared credential-free acquisition channel client
  executor.rs                 # Enforced execution boundary
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
| `toolchain` | Select the provisioned toolchain for the detected manager, or report unsupported integration |
| `prepare` | Capture native dependencies through the scoped broker and executor; return an immutable-input record |
| `plan` | Return typed command, task, artifact and report intent using captured source and prepared inputs |
| `instrument_override` | Optionally add native reporting to an exactly recognized replacement command; required evidence stays owned by the operation |
| `runtime_files` | Declare compiled-in adapter assets needed by isolated native processes |

`BuilderPlan`, `CommandSpec`, `TaskPlan`, `ArtifactSpec` and `ReportSpec` are Rust structures. A builder does not assemble arbitrary build-plan JSON. The common planner expands hooks, preserves TOML replacements, assigns action identities, binds source/dependency/toolchain identities and serializes the versioned plan. Report formats and input conversions are explicit types.

Command replacement and evidence requirements have separate ownership. A TOML override cannot remove the builder's required reports. The optional override adapter may instrument an exact known native command; the shared planner does not parse ecosystem commands or shell programs. Unknown replacements retain their arguments and receive `OYZU_TEST_REPORT` and `OYZU_COVERAGE_REPORT` destinations when those kinds have one concrete destination. Native stdout conversion applies only to native or recognized commands; arbitrary replacement output is not assumed to use the native event protocol. The shared reporting binder resolves custom declarations against captured task cwd and retains requirements for undeclared kinds. Hooks inherit the operation's report destinations. The Node adapter receives resolved destinations and owns conversion to reporter arguments.

Artifact names are owned strings, and artifacts can override the target-level version. This lets a native workspace expose multiple independently versioned package outputs without adding Cargo-specific cases to the engine. Cargo preparation uses the same captured-input interface as acquisition, recording native workspace metadata and a version-projected manifest/lock overlay; it does not require a separate execution path.

Native report intent can also identify a module and a contained file/glob under the task's working directory. Maven uses this to attribute Surefire and JaCoCo reports to reactor modules without adding Maven branches to the planner or collector. Shared report formats include native JaCoCo aggregate line counters. Prepared input snapshots include the entire acquired tree, even native coordinate directories named `target`; source checkout exclusion rules do not apply to repositories.

The executor owns process invocation and sandbox flags. A builder's planned command does not grant a host mount, credentials or network access. The broker owns request validation and upstream authorization; each acquisition adapter supplies its configured source routes. Dependency-free adapters return no prepared snapshot. Discovery-only adapters return an explicit error for unimplemented build behavior.

Native runtime code belongs to its ecosystem. The embedded Python adapter exists to invoke native package tooling and inspect native metadata inside the isolated toolchain environment. It does not own scheduling, policy decisions or bundle finalization. Further Python growth should split manager and operation modules inside `builders/python`, not add unrelated ecosystems to a global helpers directory.

Report collection is a separate engine responsibility. It retains contained raw report bytes before parsing, within the shared 16 MiB report limit. Parsing and digests refer to those retained bytes. Invalid reports fail the action but remain available for diagnosis; missing, escaping or oversized files are not copied into the bundle. Native report formats remain in `reports.rs`, rather than being implemented separately by each builder.

The collector queues executed producers until their post-hook boundary. It also drains a failed producer when that post-hook is blocked, and retains available evidence when post itself fails. Declared globs use the same typed formats and bounded capture as native reports; every matched file receives a stable identity and independent validation. Glob expansion does not follow symlinks. The scheduler owns outcome changes and downstream blocking; the collector does not run commands or choose build stages.

## Adding a builder

Implement the trait in the ecosystem module and register it once in `builders/mod.rs`. Put native inference and version projection in that adapter, reuse task constructors and shared report formats, and declare exact output identities before execution. Add native scenario verification for its artifacts and failure behavior. Do not add a new manager switch to the shared engine, let native acquisition contact arbitrary sources, or manufacture successful results for missing integrations.

Tests cover unique registrations, native ownership ambiguity, required prepared inputs, typed Python output/report intent and preservation of explicit task overrides. Existing discovery, hook, bundle and native CI scenario tests remain the behavior checks across this structural change. Cross-platform runtime support must still be demonstrated independently.
