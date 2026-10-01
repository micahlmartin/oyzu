---
id: OEP-0014
title: Ecosystem builders and executable examples
status: draft
implementation: in-progress
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0005, OEP-0006, OEP-0007, OEP-0012]
tracking-issue: null
---

# Ecosystem builders and executable examples

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

Implementation detail: [builder profiles and adapter contracts](implementation.md), [default testing and evidence](testing.md), [dependency acquisition](../OEP-0017-dependency-acquisition/README.md), and [container/Helm packaging](../OEP-0018-container-and-helm-packaging/README.md). These specify initial draft implementation choices without marking support implemented.

## Problem and outcome

Builders encode expert defaults, while runnable examples define their intended behavior. The project must demonstrate progressively harder real repositories rather than create configuration knobs speculatively.

## Builder contract

A versioned builder supplies safe discovery, native workspace/module interpretation, task providers, preparation rules, planned outputs, report adapters, execution requirements, and cacheability rules. The engine owns configuration, dependency graph validation, hooks, isolation, policy, and manifest integrity.

Discovery contributions use [specialized detectors](../OEP-0006-discovery-and-planning/detectors.md). Ecosystem-owned detectors recognize native managers/frameworks and return evidence; the shared resolver decides ownership and compatible selections. Builders consume the resolved profile to plan work. Adding a framework does not require extending a central framework-name conditional chain, and multiple compatible capabilities can coexist.

Builders SHOULD delegate ecosystem semantics to the pinned native tools. Oyzu must not reimplement Maven resolution, Cargo workspace rules, or Python package metadata interpretation with inconsistent shortcuts. Native tool invocation is allowed; invoking a separate mise executable is not.

Initial builders ship with the CLI to reduce distribution and compatibility complexity. An extension ABI and remotely distributed builders are future design work. Adding a builder requires conformance examples, supported-version boundaries, and a security review of acquisition and execution paths.

## Ecosystem obligations

| Ecosystem | Discovery and preparation | Default tasks and outputs | Important edge cases |
| --- | --- | --- | --- |
| Python | pyproject metadata, manager-specific locks, requirements files, legacy setup metadata; uv/pip/Poetry and explicit unsupported-manager detection | Native package/app build, detected tests and reports, configured lint/format tools; wheel/sdist or app/container inputs as appropriate | PEP 517 build dependencies, native extensions, multiple managers, requirements without hashes, editable installs, app entrypoints |
| Go | go.mod/go.work and native package graph | Dependency preparation, compile, test/coverage, formatting check; binaries or declared library/test outputs | Multiple main packages, cgo, private modules, build tags, cross-compilation |
| Node | package.json, selected manager lock, workspace metadata | Native scripts exposed as tasks, inferred build/test where unambiguous; application/package outputs | npm/pnpm/Yarn, lifecycle scripts, workspaces, competing locks, native modules, frontend output directories |
| Rust | Cargo.toml/Cargo.lock and workspace graph | Cargo build/test, supported coverage, fmt check and clippy where available; binaries/libraries/packages | Features, build.rs, proc macros, target triples, system libraries |
| Java | Maven pom.xml, Gradle settings/build files and wrappers, Ant build.xml | Native module/task graph, compile/test/package, supported reports; jars/wars or declared outputs | Reactor/included builds, plugins/processors, wrapper acquisition, toolchains, private repositories, arbitrary Ant target semantics |
| Docker | Dockerfile/context or explicit builder intent | Context preparation, image build, metadata/SBOM hooks and OCI image outputs | Pinned base digests, multi-stage/multi-platform, secrets, networked RUN steps, external ADD, context exclusions |
| Helm | Chart.yaml, values, Chart.lock and dependencies | Dependency preparation, lint, supported rendering/schema checks and packaging; chart archives | Chart dependencies, application/chart version distinction, OCI publication, template values needed for validation |

Every builder MUST expose an implicit `test` task. It detects and preserves an existing test framework or native test lifecycle; when none is found, it selects the builder profile's documented standard default. Test execution automatically requests JUnit XML and code coverage, with report collection into `dist/` and digest-bound manifest records. No ordinary project needs a test, output or reporting section in Oyzu configuration. The [testing contract](testing.md) defines defaults, applicability, failure behavior and conformance.

Selecting a default runner does not invent tests, an application framework, entrypoint, deployment environment or lint regime. Java/Ant and custom Node scripts especially require evidence-based discovery. An existing unrecognized runner is preserved and diagnosed rather than replaced by an unrelated fallback. Multiple plausible managers, incompatible runners or output types produce a targeted explanation and narrow override.

Automatic test/coverage/reporting tooling is part of the builder's pinned preparation inputs. Coverage collection is enabled by default for supported source instrumentation; a configured threshold is not required to collect it. No missing tool is installed from the public internet during an action. An unsupported integration remains an implementation gap, not permission to silently omit required evidence or claim full builder support.

## Containers and composition

A conventional Dockerfile often downloads dependencies in RUN instructions. Oyzu must either prepare its inputs through a supported builder integration or reject unsupported network-dependent execution under hermetic policy. Merely pinning FROM does not make the build hermetic.

An explicit application builder may produce a conventional container with pinned runtime/base inputs and discovered entrypoint. If the entrypoint or exposed runtime behavior is ambiguous, require the smallest explicit setting. Existing Dockerfiles remain authoritative within policy and execution constraints.

Helm packaging can consume an image digest from another target through a declared artifact binding. It must not silently edit source values or imply deployment. Container and chart publication are separate artifact operations.

Dockerfile consumers use `materialize` to place producer artifacts into a prepared context. The consumer selects a single target platform or a platform matrix; the engine resolves compatible producer variants and preserves ordinary Dockerfile COPY paths. EX-027, EX-029, EX-030, EX-049, and EX-050 define this agreed example contract. Chart digest-value binding is still a separate open design; it must not be confused with copying a file into a context.

## Package-manager adapter conformance

EX-051 through EX-058 exercise private dependencies across Python, Node, Java, Go, Rust, Helm and container OS packages. Every supported adapter must preserve native resolution while sharing connector authentication, broker routing, captured-input execution and credential exclusion. [The example contract](../../../examples/DEPENDENCIES.md) also records unsupported dynamic downloads and open custom-Dockerfile integration syntax. These examples do not declare that all managers are already implemented.

## Examples as the implementation contract

The [example catalog](../../examples.md) defines stable EX identifiers, expected commands, inferred facts, outputs, and acceptance links. Each implemented fixture includes minimal source, native metadata/locks, expected semantic results, supported host matrix, and a short guide. Oyzu configuration appears only when the scenario requires an override.

Fixtures run independently of the Oyzu repository's environment. Expected results compare semantic plans/manifests and observable outcomes, not incidental timestamps or log formatting. Negative examples cover ambiguity, undeclared inputs, malicious hooks, credentials, and forged trust.

A fixture progresses proposed → implemented → verified; a document listing a capability does not mean it is implemented. Future remote execution and advanced matrices require their own examples before public support claims.

## Acceptance scenarios

- BUILDER-01: Each initial ecosystem has a no-config conventional fixture and an explicit customization fixture.
- BUILDER-02: Native multi-module/workspace builds do not duplicate task execution or artifacts.
- BUILDER-03: Native package scripts and grouped implicit tasks appear in task discovery.
- BUILDER-04: Ambiguous managers, entrypoints, outputs, or unsupported report formats fail or disclose limitations precisely.
- BUILDER-05: Container and chart composition preserves artifact digests without deploying anything.
- BUILDER-06: Every supported builder passes common planning, task, sandbox, report, and cache conformance cases.
- BUILDER-07: Docker contexts contain declared file/directory artifacts at deterministic paths with compatible platform identities and no manual copy scripts.

- BUILDER-08: Each supported package-manager adapter demonstrates approved private-source preparation, target-compatible offline inputs, and credential-exclusion negative cases in standalone and managed profiles.
- BUILDER-09: Every builder lists a qualified implicit test task without TOML duplication; recognized native frameworks win over the documented fallback, and unknown custom runners are not silently replaced.
- BUILDER-10: A conventional source project produces real JUnit and coverage files plus valid dist manifest references without reporting configuration, both through build and direct test invocation; failed tests retain available reports.
- BUILDER-11: Tests absent, zero collected, skipped, failed, unsupported, disabled and missing evidence remain distinct; a missing expected report fails the test contract, and packaging cannot fabricate application coverage.
- BUILDER-12: Native modules and matrix variants retain attributable test/coverage evidence without rerunning native lifecycle tests or double-counting coverage; custom overrides retain required evidence contracts.

## Open decisions

Choose the first implementation vertical slice and exact supported manager versions. A Python/uv application followed by Go is a proposed sequencing choice, not a reduction of the agreed ecosystem scope. Do not finalize advanced configuration syntax before examples expose the minimum required controls.
