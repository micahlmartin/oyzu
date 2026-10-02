# EX-030: Python API Node frontend Go command image and chart

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Discover target-specific toolchains and grouped tasks.
- Infer native dependencies. The image's materialize reference selects the command artifact for linux/amd64 and places it at bin/server; no manual copy or duplicate dependency is needed.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu run list
oyzu build
```


The build YAML uses draft OEP syntax and contains only target intent/relationships. It is not a stable schema.

## Native checks available now

The first complete captured-build demonstration is registered as `mixed-monorepo` at the start of the `core` acceptance suite. It copies this project unchanged, runs the compiled CLI's task listing and one full build, then inspects all five targets' artifacts, reports and the Go-to-image materialization receipt. Follow [builder acceptance setup](../../../docs/reference/build-verification.md) to provision and run it. Linux captured acceptance is pending; the advanced cases below are not established by registering this check. The checked-in Go and JavaScript sources follow the implicit native formatter defaults so the first build can exercise the read-only quality gates.

With the named toolchains already installed, run:

```text
# From project/api
python -m unittest discover -s tests

# From project/web
node --test

# From project/command
go test ./...
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **target-collision:** Introduce a second target with the same output identity. Expected: Planning rejects collision before execution.

## Contract and limitations

Acceptance criteria: PLAN-02, TASK-06, PLAN-07, PLAN-08, BUILDER-07. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).

- Materialization/platform behavior follows the [agreed contract](../../MATERIALIZATION.md) and [expected context](expected-materialization.json). Image-to-chart digest-value binding remains under review.

Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
