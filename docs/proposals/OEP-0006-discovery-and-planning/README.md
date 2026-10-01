---
id: OEP-0006
title: Builder discovery and build planning
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0004, OEP-0005]
tracking-issue: null
---

# Builder discovery and build planning

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

## Problem and outcome

A conventional repository should need only `oyzu build`. One build can include several projects, native modules, containers, and charts. Configuration supplies missing intent; it should not repeat facts already present in native manifests.

## Discovery contract

Discovery MUST inspect the captured repository boundary, applicable configuration, native manifests and lockfiles, and versioned builder descriptors. It MUST NOT execute arbitrary repository scripts merely to identify a builder. Generated metadata may require a declared, sandboxed discovery action. Its inputs, toolchain, output, and effect on planning are recorded.

Each builder reports candidates with evidence, root, supported tasks, artifact kinds, native dependency relationships, and uncertainty. Deterministic rules resolve unambiguous cases. A Python library manifest does not by itself establish that the project is an HTTP application or that its desired output is a container. When intent cannot be established, Oyzu explains the ambiguity and requests a small configuration choice; it never silently invents application entrypoints.

Minimal explicit configuration remains:

```yaml
myapp:
  uses: python/app
```

An illustrative multi-project form is:

```yaml
api:
  uses: python/app
  path: services/api
web:
  uses: node/app
  path: services/web
image:
  uses: docker/image
  path: .
  depends_on: [api, web]
```

Field spelling beyond `uses` remains proposed. Dependencies express ordering and declared artifact consumption, not permission to read arbitrary sibling outputs. Native workspace/module relationships SHOULD be inferred. Explicit edges add intent without restating Maven, Gradle, Cargo, Go, or package-manager graphs. Nested projects MUST NOT be built twice by a parent native invocation and a second inferred target.

## Planning phases

1. Establish management state and repository boundary; capture source identity and dirty-state facts.
2. Resolve configuration, builder versions, source-control facts, policy snapshot, tool/dependency locks, and execution capabilities.
3. Discover targets and tasks; resolve overrides, hooks, native task ownership, and dependency edges.
4. Expand requested variants and required checks; compute versions, artifact names/types, execution constraints, and cache eligibility.
5. Validate ambiguity, cycles, output collisions, unavailable capabilities, and policy conflicts.
6. Freeze the execution plan against the source/input snapshot and begin execution.

`oyzu build --plan` is proposed as the inspection interface. Planning can require metadata or dependency resolution through approved acquisition channels; it is not guaranteed to be offline. A preliminary discovery preview MUST be labelled incomplete, rather than represented as an executable frozen plan.

The canonical semantic plan contains target/action identities, edges, selected task definitions and hooks, input and tool digests, resolved environment, artifact identities, versions, classification facts, effective policy revision, and requested reports. Timestamps, random run IDs, credentials, and incidental machine paths MUST NOT affect its semantic digest. An execution envelope carries run-specific facts separately.

Artifact names and versions are planned; final content digests exist only after execution. An artifact whose content determines its name uses an explicit placeholder binding, finalized and recorded by the engine.

## Scheduling and advanced builds

The planner expands finite matrices of supported runtime/platform/toolchain variants. Initial syntax is deferred until examples justify it. Expansion MUST detect incompatible variants, bound total actions, and include variant identity in paths and cache keys. This is declarative data, not loops, conditionals, or an expression language.

Independent graph nodes may execute concurrently subject to resource and concurrency limits. Changed-project selection uses declared inputs and transitive dependents; unknown inputs force conservative rebuilding. Full builds remain available. Native build tools own their internal scheduling unless a builder can expose a correct finer graph.

The public executor contract can later support remote execution. Remote workers must honor the same input/output and evidence contracts; remote execution is not required for the first usable release.

## Failures and visibility

Plans MUST explain why a target, scanner, variant, or hook is included and where its configuration originated. A missing required report or execution capability fails explicitly. Plan-changing source or policy updates require replanning; urgent revocation can halt execution or block publishing without rewriting the original plan.

## Acceptance scenarios

- PLAN-01: A conventional single project produces an inspectable plan without Oyzu configuration.
- PLAN-02: A mixed repository and native multi-module project produce each output once with correct edges.
- PLAN-03: Equal captured inputs and policy produce the same semantic plan digest.
- PLAN-04: Ambiguous app entrypoints, cycles, and duplicate artifact paths fail before action execution.
- PLAN-05: Matrix variants cannot overwrite one another and affected selection includes transitive dependents.
- PLAN-06: A newly introduced hook changes the planned action identity and appears in explanation output.

## Alternatives and open decisions

Reject a general pipeline DSL and configuration generated in bulk just to encode discovery. Decide supported matrix axes, artifact-edge syntax, versioned builder distribution, and the first remote executor only after executable examples establish their needs.
