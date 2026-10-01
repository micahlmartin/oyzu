# EX-042: Explicit probes for host and network access

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Build actions and hooks cannot read an undeclared host sentinel or access an unapproved loopback service.
- Development execution is not evidence of sandbox enforcement.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

This is an interaction/evidence contract. Review its input files and expected outcomes; no substitute Oyzu implementation is supplied.

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **host-file:** Supply a temporary sentinel outside captured inputs to the probe. Expected: Build action cannot read it.
- **network:** Request the harness loopback endpoint from an isolated action. Expected: Network denied.
- **symlink:** Create an escaping link in an isolated copy. Expected: Capture/output validation rejects it.

## Contract and limitations

Acceptance criteria: EXEC-01, EXEC-06. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
