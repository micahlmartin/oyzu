# EX-027: Separate host execution and container target platforms

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Produce distinct linux/amd64 and linux/arm64 image identities.
- Do not interpret a multi-platform image as proof tests executed on both architectures.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


The target's `matrix.platform` list in build.yaml is candidate syntax for review. Artifact paths and manifests must retain each target platform identity.

## Native checks available now

This is an interaction/evidence contract. Review its input files and expected outcomes; no substitute Oyzu implementation is supplied.

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **missing-executor:** Request native target tests without a matching executor. Expected: Explicit unsupported capability; no fabricated test evidence.

## Contract and limitations

Acceptance criteria: PLAN-05, EXEC-04. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).

- The matrix request is harness data. Running one native command does not verify all variants.

Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
