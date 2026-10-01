# Executable example catalog

Status: **all 48 design-contract scenarios are now checked in** under [examples](../examples/README.md). They contain project source/configuration, expected behavior, and negative cases for review. Oyzu implementation and behavioral verification remain pending.

Each row is a scenario family. Manager/runtime/platform variants have separate project roots or explicit candidate variant definitions. Managed examples contain synthetic context/evidence inputs, not a running platform implementation or real credentials. The [review guide](../examples/REVIEW.md) highlights the shape decisions to discuss.

## Catalog

| ID | Scenario | Minimal setup and action | Expected observable result | Acceptance links |
| --- | --- | --- | --- | --- |
| EX-001 | Tool install and lock | Install a Node version, commit the lock, repeat on a clean host | Exact verified distribution reused; mutable upstream bytes rejected | TOOL-01, TOOL-02 |
| EX-002 | Switching tools | Two adjacent projects select different Node/Python versions | Directory changes restore PATH/env; terminals stay independent | ENV-01, ENV-03 |
| EX-003 | Local environment overrides | Checked-in [env] plus ignored local settings | Precedence/origin visible; CI ignores local overrides by default | CFG-01, CFG-05 |
| EX-004 | Headless environment | Execute a command with no shell hook or desktop | Correct tools/env, exit code, Unicode/spaced paths on each host | ENV-04, DIST-01 |
| EX-005 | Repository trust | Enter a repository containing executable hooks | No automatic script execution before trust; changes invalidate relevant approval | ENV-02 |
| EX-006 | Go implicit tasks | Go project with no Oyzu files; run task listing and test | Compile/test/dependency/format tasks inferred; available lint explained | TASK-01, BUILDER-01 |
| EX-007 | Native npm scripts | package.json has foo and test scripts | Native scripts exposed once with native hook ownership preserved | TASK-01, BUILDER-03 |
| EX-008 | Grouped tasks | API and web targets in one build.yaml | api:test and web:test listed; ambiguous unqualified names diagnosed | TASK-06, PLAN-02 |
| EX-009 | Override plus hooks | Override api:test and define pre_test/post_test | Same order in run and build; each failure position has expected outcome | TASK-02, TASK-03, TASK-04 |
| EX-010 | Cached task with hooks | Reuse main result while noncacheable hooks remain | Effects are not skipped; changed hook inputs affect reuse | TASK-05, CACHE-02 |
| EX-011 | Python uv library | Conventional pyproject and uv.lock | Build wheel/sdist, inferred tests and reports, offline execution | BUILDER-01, EXEC-02, BUNDLE-01 |
| EX-012 | Python application/container | Conventional API with detectable entrypoint or minimal python/app intent | Application/test outputs then digest-bound image; no mandatory output/test sections | PLAN-01, BUILDER-04, BUILDER-05 |
| EX-013 | Python pip requirements | Requirements-based app and pinned dependencies | Dependency closure captured, missing hashes/entrypoint handled explicitly | EXEC-02, BUILDER-04 |
| EX-014 | Python Poetry | Poetry metadata and lock | Native resolution retained; test/coverage integration without duplicate config | BUILDER-01, BUNDLE-03 |
| EX-015 | Python legacy/native | Legacy setup metadata plus a native extension | Declared discovery/build requirements; no network or host compiler leakage | EXEC-01, EXEC-02 |
| EX-016 | Go application | Single main package and go.mod | Binary/test/coverage bundle from no Oyzu build file | PLAN-01, BUILDER-01 |
| EX-017 | Go workspace and cgo | go.work modules, cgo and alternate target | Native graph respected; declared C toolchain; unsupported target explained | BUILDER-02, EXEC-04 |
| EX-018 | Node application/package | Conventional native scripts with npm lock | Detected outputs/reports; no invented entrypoint or build script | BUILDER-01, BUILDER-04 |
| EX-019 | Node manager variants | Equivalent npm, pnpm, and Yarn fixtures | Correct lock/manager selected; competing locks produce ambiguity error | BUILDER-04, PLAN-04 |
| EX-020 | Node workspace | Workspace packages with dependency edges | Native ordering; outputs once; affected selection reaches dependents | BUILDER-02, PLAN-05 |
| EX-021 | Rust application | Cargo manifest and lock | Build/test/package and detected format/lint/report support | BUILDER-01, BUNDLE-03 |
| EX-022 | Rust workspace/features | Workspace with build.rs, proc macro and features | Declared generators/toolchains; per-variant inputs and outputs | BUILDER-02, EXEC-01, PLAN-05 |
| EX-023 | Maven reactor | Parent POM and several modules | Native reactor builds once; JUnit/JAR/WAR outputs associated to modules | BUILDER-02, BUNDLE-01 |
| EX-024 | Gradle multi-project | Wrapper with multi-project/included build | Verified wrapper/toolchain; native task graph and reports | TOOL-01, BUILDER-02 |
| EX-025 | Ant project | Conventional and unconventional Ant targets | Safe known tasks inferred; arbitrary semantics require a focused hint | BUILDER-04, TASK-01 |
| EX-026 | Dockerfile | Multi-stage image with pinned prepared inputs | Network-disabled execution, OCI output; external ADD/download rejected | EXEC-01, BUILDER-05 |
| EX-027 | Container platform variants | One application for supported target architectures | Platform-distinct artifacts; no host/execution/target confusion | PLAN-05, EXEC-04 |
| EX-028 | Helm chart | Chart.yaml with locked dependencies | Dependency preparation, lint/render checks where valid, chart package | BUILDER-01, BUILDER-05 |
| EX-029 | Image and Helm composition | Application, image, chart targets with digest binding | Chart references produced image without source mutation or deployment | BUILDER-05, BUNDLE-01 |
| EX-030 | Mixed monorepo | API, frontend, shared package, image and chart | One graph, qualified tasks, inferred and explicit artifact edges | PLAN-02, TASK-06 |
| EX-031 | Generated source | Declared schema/code generator used by two targets | Generator action inputs/outputs captured; correct invalidation | PLAN-06, CACHE-02 |
| EX-032 | Compatibility matrix | Tests across supported runtimes without repeated YAML | Finite variants, collision-free output/report paths and constrained expansion | PLAN-05, BUNDLE-01 |
| EX-033 | Affected builds | Edit leaf/shared source in a mixed graph | Conservative transitive rebuilds; unknown inputs force rebuild | PLAN-05, CACHE-02 |
| EX-034 | OCI cache in CI | OSS job invokes Oyzu with OCI cache destination | Automatic lookup/upload, no custom cache service/save-restore scripts | CACHE-01, CACHE-03, CACHE-06 |
| EX-035 | Local cache and trust | Local result uploaded then consumed by verified CI | Content reuse and producer evidence evaluated; no production trust laundering | CACHE-04, REL-06 |
| EX-036 | Bundle failure/integrity | Fail tests, interrupt execution, mutate output after finalization | Accurate partial manifest; corrupted bundle cannot publish | BUNDLE-02, BUNDLE-04, BUNDLE-06 |
| EX-037 | Snapshot versus release | Same source under local, unverified CI, and verified CI facts | Policy-specific versions/destinations/signing; local production denied | REL-01, REL-02, REL-03 |
| EX-038 | Publishing retries | Several artifacts with partial registry failure | Idempotent retry and receipts; no mutation of original manifest | REL-04, BUNDLE-05 |
| EX-039 | Managed tool install | Protected machine config, one approved connector/catalog | Headless login, approved version installs through corporate route | AGENT-01, CONN-05, PROTO-02 |
| EX-040 | Native package proxy | pip/npm point at local agent; token expires | Direct upstream streaming, refresh, scoped routes, no upstream credential exposure | AGENT-02, AGENT-03, CONN-03 |
| EX-041 | Management outage/logout | Agent or platform unavailable and user logs out | No public fallback; valid offline scope distinguished from expired grants | CFG-02, AGENT-04, PROTO-05 |
| EX-042 | Host and network escape | Build/hook attempts home read, symlink escape or network download | Sandbox denies and bundle records failure | EXEC-01, EXEC-06 |
| EX-043 | Policy-driven checks | Same repository under two public mock policy revisions | New required scanner appears in plan; unsupported capability fails | PROTO-03, PLAN-06 |
| EX-044 | Service integration test | Explicit test needing a prepared local service or approved endpoint | Service scope and weaker evidence disclosed; no blanket hermetic claim | EXEC-05 |
| EX-045 | Desktop independence | CLI-only flow then attach/close optional desktop | Agent/build continues; protocol compatibility and redaction verified | DIST-01, DIST-02, DIST-03 |
| EX-046 | Cross-platform shell lifecycle | Activation/profile install/remove on supported shells | Idempotent profile edits, environment restoration, signal/exit behavior | ENV-04, ENV-05, DIST-05 |
| EX-047 | Backend compatibility | Pinned supported mise backends with synthetic acquisition servers | No separate mise executable; all enterprise acquisition routes accounted for | MISE-01, MISE-04, TOOL-04 |
| EX-048 | Source-control evidence | GitHub/GitLab protected refs, tags, fork PRs, missing permissions | Unknown/protection facts accurate; verified identity required for release | CONN-06, REL-02, REL-05 |

## Fixture contract

An implemented example has a short README, minimum native source/manifests/locks, only necessary Oyzu configuration, exact commands, expected semantic plan/bundle assertions, negative cases, host/target support, and a record of verification. No repository-wide hidden env or credentials may be required.

The checked-in layout is `examples/<area>/<scenario>/`, with expectation data outside user-facing project roots. Source is locally authored and minimal. Secret/identity examples use synthetic metadata.

The smallest build example contains native project files and **no build.yaml**. A customization example adds only the field necessary to explain its exception. Task examples explicitly demonstrate `oyzu run list`, qualified names, overrides, and naming-based hooks. No fixture should normalize boilerplate tests/outputs configuration.

Assertions compare meaningful graph edges, tool identities, artifact versions/digests, report presence/state, hook ordering, denied operations, and evidence origins. Golden files ignore run timestamps and other incidental fields. They must not merely duplicate internal implementation output.

## Progress and host coverage

Current state is design-contract authored. A scenario becomes behaviorally implemented when its Oyzu integration and executable assertions exist; it becomes verified only with recorded results on its advertised host/toolchain matrix. Native source and generated package-manager locks do not imply Oyzu support. Verified on Linux does not imply Windows/macOS.

Organizational policy tests use the public mock contract. Private server integration adds its own tests without becoming a prerequisite for the public examples.

## Traceability and prioritization

Acceptance prefixes resolve through the [proposal index](proposals/README.md): DOC→0001, CFG→0002, MISE→0003, TOOL→0004, TASK→0005, PLAN→0006, EXEC→0007, ENV→0008, AGENT→0009, CONN→0010, CACHE→0011, BUNDLE→0012, REL→0013, BUILDER→0014, DIST→0015, PROTO→0016.

The [delivery plan](delivery.md) sequences examples into small complete slices. A proposed example is a requirement candidate for review, not an implementation issue already created.
