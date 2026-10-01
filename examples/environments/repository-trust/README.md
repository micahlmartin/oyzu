# EX-005: Untrusted repository tasks do not run on entry

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Entering the directory or listing tasks never creates marker.txt.
- Explicit execution follows repository trust approval; material command changes invalidate that approval.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu run list
oyzu run mark
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

This is an interaction/evidence contract. Review its input files and expected outcomes; no substitute Oyzu implementation is supplied.

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **modified-task:** Change the task command after approval. Expected: Executable trust is reevaluated before execution.

## Contract and limitations

Acceptance criteria: ENV-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
