# Default testing and evidence contract

This specifies DEC-029 under OEP-0014 and OEP-0012. The product requirement is agreed; runner choices and implementation details remain draft. It does not claim every integration is implemented. See [implementation status](../../implementation-status.md).

## Universal behavior

Every builder MUST declare an implicit `test` task, including Docker and Helm. A target named `api` exposes `oyzu run api:test`; ordinary single-target lookup supports `oyzu run test`. Discovery lists the task even when there are no tests yet, and explains its selected native runner or default and any capability limitation. No TOML task, YAML test section, report path or coverage toggle is required for a conventional project.

`oyzu build` includes the test operation and its evidence contract. Direct test invocation uses the same framework selection, hooks, report adapters and collection semantics. Native lifecycle ownership remains authoritative: a Maven or Gradle invocation that already executes tests is not followed by a duplicate test run just to populate a separate stage. The plan records which action actually owns the evidence.

For source-bearing projects, tests MUST request both JUnit XML and measured code coverage by default. A coverage threshold is a separate policy decision; no threshold does not mean no coverage collection. Framework-native reports may be retained in addition to the standard JUnit result. Coverage retains a supported machine-readable format and its metric/source mapping rather than forcing every ecosystem into a lossy universal format.

## Deterministic selection

Framework recognition is implemented by specialized [detectors](../OEP-0006-discovery-and-planning/detectors.md), separate from command planning. Detectors report native evidence and suite scope; the resolver selects or composes frameworks before the builder requests test and coverage integrations. Registration order and arbitrary numeric scores never choose a framework. Distinct suites may coexist without forcing one global winner.

Selection follows this order:

1. An explicit same-name Oyzu task override supplies the body, while inheriting the builder's required reports.
2. Recognized native test scripts, lifecycle tasks and framework configuration select the existing framework and its native semantics. Declared/locked dependencies and test source conventions provide supporting evidence.
3. With no existing test mechanism, use the profile's documented default. A default can discover zero tests; it must not generate artificial passing test cases.

The adapter records the selection reason, pinned runner/reporter versions, relevant native configuration and required report formats in the plan. Native scripts remain wrapped by their package manager so lifecycle behavior is preserved. Do not parse arbitrary shell programs to infer a runner. An existing unknown custom test command is not evidence that no runner exists: retain it and request the smallest supported reporting override. Multiple native test suites may coexist when the adapter can model them explicitly; genuine ambiguity receives a targeted diagnostic.

No LLM, online framework guessing or project-source rewrite participates in selection. Runner, instrumentation and converter dependencies are captured during preparation through approved sources. Actions execute with the prepared inputs and no direct download fallback. Generated reporter configuration belongs in the private execution workspace.

## Initial default profiles

These are implementation targets, not universal compatibility claims. Exact tool versions and supported manager ranges belong to versioned builder descriptors.

| Builder | Preserve when detected | Fallback and evidence obligation |
| --- | --- | --- |
| Python | pytest, unittest and recognized native test tasks | pytest discovery with a pinned coverage integration; preserve unittest semantics through a supported adapter. Emit JUnit and application-source coverage. |
| Go | Native package/workspace tests and configured build tags | Native Go test runner, machine-readable events converted to JUnit, and native coverage profiles. |
| Node | Native package scripts and recognized Node test, Jest or Vitest configuration | Node's built-in test runner for its supported source/runtime profile, with a pinned JUnit/coverage adapter. Unsupported transpilation or browser requirements need an appropriate integration, not a false zero-test success. |
| Rust | Cargo workspace tests, configured nextest suites and separate doctest semantics | A pinned nextest/JUnit profile plus compatible LLVM coverage integration; retain Cargo-owned suites that nextest does not execute. Test and coverage scope must disclose any unsupported suite or target. |
| Maven | Native Surefire/Failsafe lifecycle and selected test providers | Conventional native test discovery; a pinned JUnit provider where none is established and a JaCoCo integration. Preserve existing providers such as TestNG rather than rewriting test source. |
| Gradle | Native test suites/tasks, framework and source-set metadata | Conventional JVM test/JUnit profile with a pinned JaCoCo integration. Attribute reports to the native project and suite. |
| Ant | Recognized native test targets, framework and source/class directories | A pinned JUnit/JaCoCo adapter when conventional layout is unambiguous. Custom layouts require a narrow task/report override, not an arbitrary host command. |
| Docker | Declared image tests and evidence from packaged application producers | Default deterministic image structure/configuration assertions, reported as JUnit cases. Entrypoint/runtime smoke tests require established runtime intent and capabilities; do not deploy or start arbitrary services to invent tests. |
| Helm | Recognized chart test suites and their required capabilities | Default contained render/schema validation tests with JUnit assertions. Cluster-dependent hooks remain distinct integration tests; never contact an ambient cluster. |

Fallback tooling is builder-provided infrastructure. It does not require developers to add generic test/report boilerplate to their native project files. It also does not authorize silently changing application dependencies or replacing a framework already selected by the project.

## Coverage subjects and truthful outcomes

Coverage measures an identified source set. Exclude test harnesses and third-party dependencies by default; preserve explicit native scope and disclose exclusions. Reports identify the captured source, module, suite and execution/target variant. Unsupported instrumentation, missing source maps or inability to execute a cross-compiled target are visible limitations, never measured zero or full coverage.

An image packaging a tested application references that producer's source coverage and exact artifact digest. Image tests assess the resulting image; they do not claim to have remeasured the application's coverage. A pure Dockerfile or Helm chart has no general application-code coverage denominator. Record that applicability fact and reason, plus any supported domain-specific measurement under its own metric. Do not label chart assertion counts or successful Dockerfile instructions as application line coverage. Every builder therefore accounts for coverage, including cases where no valid source-coverage file can exist.

The semantic outcomes MUST distinguish:

- No tests detected under the selected discovery convention.
- A selected runner collecting zero tests, versus tests explicitly skipped.
- Tests passing or failing, including runner crashes and incomplete reports.
- Coverage collected, unsupported for a requested source profile, inapplicable to a packaging-only subject, explicitly disabled where policy allows, missing despite being expected, or invalid.

No-tests and coverage-inapplicable outcomes carry reasons and cannot satisfy a policy requiring actual tests or application coverage. A fallback runner remains callable when no tests are present; absence does not become an unavailable task. If the native runner emits an empty JUnit report, retain its zero-test result without relabeling it as passed evidence. Do not synthesize a coverage percentage or a report claiming test execution when the action never ran.

For an applicable supported test operation, JUnit and coverage collection are required by default. Missing or malformed expected files fail the evidence contract even if the command exited zero. Preserve a failed native exit even if partial reports contain passing cases. Collect available reports on test/hook failure, cancellation or interruption whenever safe persistence remains possible; missing files still receive honest outcomes. Policy can separately require tests to exist, impose coverage thresholds, or deny otherwise permitted opt-outs.

## Bundle collection and direct runs

The engine assigns portable, collision-free paths under `dist/reports/<target>/<variant>/...`. Each collected JUnit or coverage file receives a manifest record with kind, format, relative path, digest, size, producing action, target/variant, source identity, collection status and normalized summary. Non-collected evidence has an explicit reason and no fabricated path/digest. Framework or multi-module outputs may yield several files; the manifest remains the authoritative inventory.

Direct `oyzu run <target>:test` publishes a test-only dist bundle through the same transaction/retention mechanism as builds. It must not append to an already finalized build manifest, mix old coverage with new results, or pretend that packaging/build actions ran. Development-mode source/environment facts remain distinguishable from captured hermetic builds, and local execution does not acquire trusted CI status merely by producing JUnit. This direct-run bundle behavior is required design work, not a claim about current task-runner implementation.

Collectors preserve safe original files before normalizing them, validate containment and bounded parsing, and retain partial failure evidence. Module totals and matrix variants are not blindly summed; combining coverage requires identical source identities and a supported metric-aware merge. Image/chart composition references producer evidence without copying it into a second purported measurement.

OEP-0012 owns the record serialization. Existing draft schemas and implementation must be extended and tested where needed for explicit applicability, selection provenance and test-only invocation context; this companion does not introduce unvalidated public field names. Task overrides keep the existing finite `reports` escape hatch and cannot remove mandatory evidence by replacing the command body.

## Implementation and acceptance

Builders own framework detection, native command/report instrumentation, default profile and applicability facts. The shared planner owns report obligations; collectors own bounded capture, conversion, manifest references and integrity. Reuse registered report formats/converters across builders and keep native integration under its ecosystem module. No manager switch or framework-specific logic belongs in the scheduler.

BUILDER-09 through BUILDER-12 and BUNDLE-07 through BUNDLE-09 require executable cases for each supported profile: existing framework, no-config fallback, no tests, passing/failing tests, real source coverage, missing/malformed reports, overrides/hooks, direct invocation, modules and platform variants. Docker/Helm additionally prove real default assertions and honest coverage applicability/producer links. Discovery success alone cannot satisfy these requirements. Keep exact tested ranges and implementation gaps in the status document until the cases pass.
