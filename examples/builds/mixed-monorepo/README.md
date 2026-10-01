# EX-030: Python API Node frontend Go command image and chart

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Discover target-specific toolchains and grouped tasks.
- Infer native dependencies and add only cross-artifact relationships that cannot be inferred.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu run list
oyzu build
```


The build YAML uses draft OEP syntax and contains only target intent/relationships. It is not a stable schema.

## Native checks available now

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

Acceptance criteria: PLAN-02, TASK-06. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).

- Container assembly requires a Linux target binary supplied to the declared Docker context path. Artifact binding awaits Oyzu; no native combined pipeline is supplied.

Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
