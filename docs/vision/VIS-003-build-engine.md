---
id: VIS-003
status: draft
updated: 2026-10-01
---

# Build engine and builders

> Vision draft consolidating agreed product direction. No feature is implemented by this document. Unresolved implementation choices are identified explicitly.


The build engine converts project intent, builder knowledge, captured source, and applicable policy into an explainable execution graph. It must handle a conventional repository without a build file.

## Responsibilities

Discovery identifies targets, package managers, native build systems, task providers, and relationships. Builders produce typed operations with inputs, outputs, dependencies, tool requirements, reports, and isolation needs. The engine resolves a plan, acquires missing inputs, executes actions, and finalizes evidence.

Default workflows can include dependency preparation, compilation, tests, formatting checks, linting, scanning, and packaging. Builders select appropriate operations; they do not run every discovered task. `format` may edit sources when directly invoked; builds check formatting against a captured snapshot.

Native build systems remain authoritative where appropriate. Maven reactors and Gradle workspaces should not be manually reconstructed in YAML. Delegating an entire build as one action provides less cache granularity than an adapter with internal action knowledge, and diagnostics must say so.

## Minimal configuration

```yaml
api:
  uses: go/app
  path: services/api
web:
  uses: node/app
  path: apps/web
```

No mandatory test/output sections are introduced. Undiscoverable relationships or entry points may require focused exceptions. Existing dependencies are inferred before adding configuration.

## Advanced capabilities

The graph model supports multiple targets, generated code, affected selection, content-based caching, parallelism, constrained matrices, typed artifact dependencies, cancellation, and bounded dynamic discovery. Remote execution is an architectural extension point; it is not assumed implemented.

Matrix variants distinguish test compatibility from artifact variation. Host, execution, and target platforms are separate. Debug/optimized modes are not release authorization.

## Guarantees

Inputs are immutable snapshots, tools are resolved and verified, and build actions cannot read arbitrary host files or reach the external network. Unsupported isolation must fail rather than silently downgrade. Tests needing external systems require explicit authorization and appropriately limited evidence.

Plans describe intent; manifests describe actual outcomes, including reused results and failures. Overrides and hooks remain subject to operation contracts and policy.

## Success

The examples cover all agreed ecosystems and progress from conventional projects to mixed monorepos, matrices, generation, and image/chart relationships. Each example has measurable expected outputs, meaningful failure cases, and a declared proposed/verified status.

## Open questions

OS isolation implementations, remote execution protocol, resource scheduling budgets, and dynamic graph limits require technical prototypes. A deterministic plan cannot promise exact future external facts or inspect generated content without a declared discovery action.
