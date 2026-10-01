# Builder and package-manager implementation profiles

## Shared builder interface

A versioned descriptor declares builder id, supported native manifest/manager/tool ranges, host/execution/target capabilities, static discovery evidence, ownership rules, implicit tasks, output contracts, report integrations and acquisition adapters. Interface methods: `discover(SourceView)`, `describe_tasks(TargetContext)`, `prepare_requests(Variant)`, `plan_actions(PreparedContext)`, `collect(ActionOutputs)` and `validate_artifacts(CollectedOutputs)`. Discovery only reads captured inputs; executable metadata uses a declared preparation action. Methods cannot fetch network content directly or access ambient credentials.

Candidate confidence is a finite certainty category (explicit, conventional, ambiguous), never an LLM score. A valid native workspace is one owner with module attribution; explicit duplicate owners fail. Descriptor identity enters plan/action keys. Start with built-in compiled adapters and native processes; no arbitrary downloaded builder code in v1alpha1.

The [detector companion](../OEP-0006-discovery-and-planning/detectors.md) refines discovery into observations and resolution. Specialized ecosystem detectors implement the crate-private detection interface and return typed evidence; the resolver combines compatible roles and rejects conflicting ownership. Builder task/preparation planning consumes the resolved profile, without independently repeating manager or framework inference. Static detection cannot mutate a target or execute native commands; deferred executable metadata belongs to declared preparation.

Every profile uses OEP-0017 preparation and OEP-0007 execution. Native command forms below describe adapter behavior, not mandatory project scripts. Select flags through a tested capability matrix for the pinned tool version; an unsupported version fails precisely. Runtime tools, reporting plugins and native compilers are all captured inputs. Initial manager support must be advertised by exact tested ranges, not all historical/future releases.

Every profile also follows the [default testing and evidence contract](testing.md): always expose `test`, preserve a detected native framework or choose the profile's standard default, and collect JUnit plus applicable code coverage into dist without configuration. The following integration limitations describe work still required; they do not make automatic reporting optional for a supported profile.

## Python: uv, pip, Poetry and legacy

Discovery: pyproject plus one supported lock selects its manager; explicit native manager tables may disambiguate preparation before a lock exists. Multiple conflicting locks fail. Requirements files select pip only when no stronger conflicting metadata exists. setup.py without modern metadata selects the legacy adapter, whose metadata execution is isolated. A package with conventional build metadata is a library; app intent requires an entrypoint convention or python/app. Never infer a web server from an arbitrary dependency name.

Preparation captures the full resolved graph including optional groups selected by native configuration, test/report tooling, PEP 517 requirements, sdists and target wheels. Build backend dynamic requirements run without upstream credentials. Resolve interpreter ABI, libc, native compiler and system library requirements for each target. Path/editable dependencies must be within captured source or declared imports. Production packaging does not ship developer editable references.

uv uses its locked sync/export/build capabilities with networking disabled; pip installs from an exact prepared wheelhouse using no-index and applicable hash checks; Poetry preserves native locked groups and install/build semantics. Do not claim a generic wheelhouse replaces every manager's lock semantics. Build wheels/sdists through the declared backend once. A source-only app prepares a runtime layout rather than fabricating a publishable wheel. Ambiguous outputs require a narrow builder declaration.

Tests: detected pytest gets JUnit XML and coverage enabled by default through pinned integrations; unittest uses an explicitly supported reporter adapter. With no existing framework, default to the profile's pytest discovery without creating tests or modifying the source project. No detected tests yields not-detected and leaves the implicit task visible. Ruff/Black/other checks run only when native configuration or explicit tasks establish intent; formatting in build mode is check-only. Coverage absence is unsupported/disabled/missing as appropriate, not zero. EX-011–015 and EX-051 cover the profiles, including legacy native compilation and incompatible wheels.

## Go modules, workspaces and cgo

Discovery reads go.mod, go.work and package structure; package metadata queries run with captured module inputs and network denial. go.work owns its modules. A single main package gives an unambiguous app binary; multiple mains give named artifacts derived from package paths and no assumed primary. Native module graph determines intermodule order. Libraries still get compile/test evidence without a fictional executable.

Preparation captures module archives/metadata/go.sum, approved VCS dependencies and toolchain. GOPROXY routing and private checksum configuration must prevent direct fallback/public path disclosure under managed policy. cgo requires captured C compiler/sysroot/library dependencies matching execution and target ABI. Cross compilation is allowed only through a supported descriptor; it does not prove target tests ran.

Actions use supported go build/test/list forms with -mod=readonly and prepared module/cache directories. Test JSON is preserved and converted to JUnit by a pinned converter; coverage profiles map to the captured source identity. gofmt checks compare formatting without changing source; lint is configured tooling, not an invented installation. Executable mode and GOOS/GOARCH/CPU flags are output metadata. EX-016–017, EX-027/029/049 and EX-054 cover single/multiple binaries and private modules.

## Node: npm, pnpm and Yarn

Native packageManager and lock select an exact manager; conflicts fail rather than preferring npm. Workspaces belong to one manager ownership graph. package.json scripts become tasks, retaining native lifecycle semantics. A script named foo appears as foo; it does not automatically enter `oyzu build`. Build/test/lint/format-check scripts are selected by the descriptor's known intent mapping. Unknown source-output directories or runtime entrypoints require focused intent; do not run every script or infer dist universally.

Preparation captures archives, lock integrity, workspace packages, manager tooling and supported lifecycle downloads. Native addons use target-compatible compiler/runtime headers; browser binaries are separate acquired assets. Store layouts remain manager-native, including pnpm links and Yarn PnP. Reject external workspace links and manager plugins that cannot run under declared constraints. Dependency scripts have no upstream tokens.

Use frozen/immutable install modes in isolated writable stores backed by captured content and denied networking. npm may run ci, pnpm frozen-lockfile/offline installation, Yarn immutable installation using its supported offline cache behavior; exact flags are version-tested. Detect runner configuration (Node test, Jest, Vitest, etc.) and add only a supported pinned reporter integration. Custom test commands without recognized reporting produce explicit unsupported evidence unless the user declares output paths. Do not conflate script exit success with test counts. EX-018–020 and EX-052 cover scripts, workspaces and managers.

## Rust/Cargo

Cargo metadata/lock/workspace defines owner, packages, features and native artifact kinds. Metadata invocation uses locked prepared inputs. The selected feature set is part of the variant/action identity; default features follow Cargo unless native/explicit configuration selects otherwise. build.rs and proc macros execute on the execution platform while libraries/binaries follow target triples. Native system dependencies and code generators must be captured.

Prepare alternate/sparse registry metadata, crate archives, Git sources and exact toolchain components through approved routes. Native credential providers participate only in acquisition, never exposing upstream tokens to build scripts. cargo build/test use frozen/offline semantics and a private target directory. Collect native artifacts via machine-readable messages instead of guessing file extensions. One bin may be primary; multiple bins require artifact selection for materialization.

Stable toolchain reporting limitations are explicit: use a supported pinned nextest/converter integration where selected, not undocumented nightly flags. Cargo test exit status alone cannot manufacture JUnit details. Coverage requires a pinned compatible instrumentation/report tool; lack of support is reported. fmt check and clippy run where configured/available under the profile; a native lint failure remains failure. EX-021–022 and EX-055 cover features/proc macros/registry isolation.

## Java: Maven, Gradle, Ant/Ivy

Maven's parent/reactor POMs, Gradle settings/included builds and Ant build.xml define native ownership. Maven reactor or Gradle multi-project build normally executes once; attribute module artifacts/reports after native completion. Arbitrary Gradle/Ant configuration can execute code, so project graph evaluation occurs only in declared constrained preparation/execution. Static task listing must not execute an unrestricted build script.

Prepare wrapper distributions with verified identities, JDK toolchains, application dependencies, parent BOMs, plugins, buildscript classpaths, annotation processors and Ivy requirements. Default discovery cannot guarantee Maven dependency:go-offline or Gradle dependency queries capture every plugin's dynamic fetch. Completeness is verified by denied-network execution; missing inputs name the plugin/phase. Gradle daemon state is private to the run or otherwise proven compatible/credential-free. Native shared result-cache reuse is disabled until producer provenance is preserved by its adapter.

Maven uses offline native lifecycle goals, normally verify; Gradle uses offline supported build/check tasks with the selected wrapper; Ant invokes known package/test targets and reports ambiguity for custom names. Ant alone has no universal resolver; recognized Ivy is supported separately. Preserve JAR/WAR/classifier outputs from native metadata, including module versions. Collect Surefire/Failsafe, Gradle and supported Ant JUnit XML; JaCoCo and equivalent coverage only with a declared pinned integration. No permanent source build-file rewrites to inject reports. EX-023–025 and EX-053 cover ownership and private build plugins.

## Docker and Helm

[OEP-0018](../OEP-0018-container-and-helm-packaging/README.md) specifies optional app packaging, Dockerfile contexts, dependency mounts, bases, OCI layout/index outputs, platform propagation and typed Helm image bindings. Preserve that contract rather than implementing a second task/policy system inside Docker. Helm packages charts without deploying; chart dependencies use the same acquisition boundary. EX-026–030, EX-049–050 and EX-056–058 are the review fixtures.

## Custom tasks and report escape hatches

Standard behavior requires no tests/output YAML. When discovery cannot identify custom task outputs, users may set the task's finite input/output paths in TOML and select a registered report format through a task `reports` array of `{kind, format, path}` records. Paths are contained globs, formats are adapter identifiers, and unknown formats fail. This adds data only; no report parser code or expressions are embedded in config. Mandatory policy check subjects/thresholds remain engine-owned. This proposed field must be covered by a concrete fixture before its first user-facing implementation.

Builder-specific exceptions have to justify a new field against an example. Do not expose native tool options one by one as Oyzu configuration; native project manifests remain authoritative. Ambiguity diagnostics identify the missing fact and accepted narrow override.

## Conformance and rollout

For every advertised manager/tool range: zero/minimal-config project; native workspace; private dependencies; missing/changed lock; offline closure; source/target variation; native report pass/fail/missing; overridden task/hook; incremental key change; credential canary; unsupported capability; cancellation and partial bundle. Declare host support separately from target support. Native command success is insufficient without Oyzu graph, sandbox, evidence and bundle checks.

Implement uv/Go vertical slices first but retain the full ecosystem obligation. Gate each adapter independently; unsupported managers must not fall through to an unsafe generic shell runner. BUILDER-01–12 plus DEP-01–07, BUNDLE-07–09 and PACK-01–08 form the graduation checklist. Wrapper binaries, dependency locks and generated digests for private fixtures must come from native tools against the synthetic fixture sources, never invented values.
