# EX-032: One project tested across runtime versions

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Expand a finite requested Node matrix without copying project task definitions.
- Keep test reports separated by runtime variant; runtime tests do not automatically imply distinct distributable artifacts.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


The target's `matrix.node` list in build.yaml is candidate syntax for review. It is deliberately finite data; it introduces no expression language or repeated task definitions.

The experimental implementation now expands this Node/npm matrix with separately provisioned runtimes. See [runtime matrix builds](../../../docs/reference/runtime-matrices.md) for provisioning, evidence and limitations. The captured CI group exercises both exact runtimes and failure cases; its observed result is tracked in [implementation status](../../../docs/implementation-status.md). This example remains a design contract until its acceptance is demonstrated, and does not imply that other runtime/platform axes are supported.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
node --test
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **unsupported-variant:** Request a runtime outside package engines. Expected: Explain unsatisfiable variant before scheduling.

## Contract and limitations

Acceptance criteria: PLAN-05, BUNDLE-01. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).

- The matrix.json expectation and candidate build.yaml describe the same two variants. Running one native command does not verify both variants.

Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
