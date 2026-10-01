# EX-028: Helm packaging with a local library dependency

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Prepare the chart's local dependency and honor its generated lock.
- Lint/render/package charts; never deploy to a cluster.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
helm dependency build chart

# From project
helm lint chart

# From project
helm template example chart
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **invalid-values:** Set replicaCount to 0. Expected: Schema validation fails before packaging success is reported.

## Contract and limitations

Acceptance criteria: BUILDER-01, BUILDER-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
