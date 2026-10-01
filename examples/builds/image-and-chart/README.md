# EX-029: Application image and chart artifact relationship

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Build image and chart targets with an explicit dependency.
- Bind the actual image digest in generated chart values; preserve original values.yaml.
- Artifact-binding syntax remains a design decision; binding intent is fixture data outside the project.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


The build YAML uses draft OEP syntax and contains only target intent/relationships. It is not a stable schema.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project/app
go test ./...

# From project/chart
helm lint .
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **digest-mismatch:** Attempt publication with a different image digest. Expected: Reject mismatched evidence/binding.

## Contract and limitations

Acceptance criteria: BUILDER-05, BUNDLE-01. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).

- Container assembly requires a Linux target binary supplied to the declared Docker context path. Artifact binding awaits Oyzu; no native combined pipeline is supplied.

Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
