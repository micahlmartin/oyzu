---
id: OEP-0007
title: Source capture and hermetic execution
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0006]
tracking-issue: null
---

# Source capture and hermetic execution

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

Implementation detail: [v1alpha1 implementation contract](implementation.md). This companion resolves the initial engineering defaults below; it remains draft, and does not imply maintainer acceptance or verified platform support. Where an older paragraph leaves an implementation choice open, the companion is the proposed initial resolution.

## Problem and outcome

A reproducible plan is insufficient if commands can download undeclared dependencies or read a developer's home directory. Execution must be constrained to captured source, acquired inputs, declared environment, and controlled outputs.

## Input preparation and execution boundary

Preparation can fetch tools, dependency metadata, packages, base images, and required scanner databases through approved sources. Those inputs are verified and captured before network-isolated actions use them. Tool locks and ecosystem dependency locks have distinct responsibilities; Oyzu MUST preserve native lock semantics rather than claiming a tool lock also locks application dependencies.

The source snapshot includes tracked files and explicitly allowed local additions. Local dirty/untracked content is represented by content digest; ignore rules and explicit inclusions define the boundary. Trusted CI eligibility requires verifiable source identity and policy-compliant checkout evidence. Symlinks escaping the boundary, inaccessible submodules, and external workspace references require declared imports or fail.

An action receives read-only inputs, a writable work area, a bounded output area, a minimal environment, and explicitly scoped temporary paths. It cannot read arbitrary home, registry, filesystem, SSH, or cloud credential state. Oyzu-provided tool shims and executables refer only to the resolved toolchain. Commands cannot evade acquisition controls by running an installer from a build hook.

Clocks, randomness, locale, timezone, host architecture, filesystem ordering, and absolute path embedding can affect output. Builders SHOULD normalize supported sources of nondeterminism. The system reports the isolation and reproducibility properties actually achieved; sandboxing alone does not prove bit-for-bit reproducibility.

## Cross-platform executor contract

The executor advertises capabilities: filesystem isolation, network denial, read-only inputs, process containment, resource accounting, and platform/architecture support. The planner requires the capability set for each action. Missing capabilities cause an actionable error; Oyzu MUST NOT silently retry the same build on an unrestricted host.

A Linux container executor is a candidate initial implementation. Linux-native isolation, macOS virtualized execution, and Windows-native or virtualized execution require proof-of-concept validation before a runtime is selected. Native Windows and macOS artifacts must not be advertised as supported merely because Linux containers run on those hosts.

`oyzu run` can also execute ordinary developer tasks in a development environment. Its output MUST identify that execution context. Reusing the task definition inside `oyzu build` subjects it to the build executor's restrictions. An unsandboxed local task cannot provide evidence equivalent to a constrained build action.

## Ecosystem preparation

Builders turn acquired dependency stores into offline inputs: Python wheels/sdists and build requirements, Go module content, Node package content, Rust crates, Java artifacts/plugins, Helm dependencies, and container base layers. Build-time backend dependencies and native compilers are included. A manager's offline flag is useful but is not the security boundary.

Some builds discover further dependencies only by executing code. Such resolution must be a declared preparation action with limited network destinations and captured results, followed by a final locked execution plan. If dependency closure cannot be captured, the build's limitations are explicit and relevant eligibility checks fail. It must not quietly regain internet access.

Tests needing external services use declared service inputs or a separately identified integration-test action with constrained network access. Such actions record their weaker reproducibility properties. Policy decides whether their evidence is sufficient for a specific requirement; they are not described as hermetic.

## Secrets, hooks, and outputs

Most compile/test actions need no secrets. Necessary secrets are short-lived handles supplied to the minimum action scope, never copied into the plan or build bundle. A secret-dependent action is noncacheable unless its safe caching semantics are explicitly established. Redaction cannot guarantee prevention of deliberately encoded exfiltration, so egress and artifact publication remain controlled.

Hooks inherit the primary task's sandbox and declared input rules. User task overrides do not disable engine enforcement. Output collectors reject path traversal and escaping symlinks. Interrupted actions cannot publish partially completed cache entries.

## Credential-free dependency execution

[Private dependency examples](../../../examples/DEPENDENCIES.md) define a shared acquisition boundary across native package managers and container packaging. Upstream credentials remain in the broker. Project and dependency scripts receive captured content, never upstream credentials. A preparation session may access approved routes through a job-scoped capability; session material cannot persist into execution outputs. Dynamic downloads, build plugins and OS package scripts require captured supported inputs or an actionable failure.

## Acceptance scenarios

- EXEC-01: An action attempting a public download or home-directory read is denied.
- EXEC-02: A prepared Python/Go/Node/Rust/Java build succeeds with action networking disabled.
- EXEC-03: Editing source after planning does not silently change the frozen build inputs.
- EXEC-04: Unsupported host isolation fails with an explanation of the missing capability.
- EXEC-05: External-service tests are visibly distinct from hermetic actions.
- EXEC-06: A hook has the same isolation boundaries as the primary task.

- EXEC-07: Private dependency acquisition keeps upstream credentials outside project/dependency execution and all exported outputs; incomplete input captures never trigger network fallback.

## Open decisions

Select executor implementations, container/runtime dependencies, allowed local source inclusion defaults, and initial service-test support through measured cross-platform prototypes. Define reproducibility verification as a separate repeated-build capability, not a blanket product claim.
