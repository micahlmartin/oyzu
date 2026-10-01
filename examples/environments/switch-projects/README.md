# EX-002: Switch between independent tool environments

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- project-a selects Node 22; project-b selects Node 24.
- Leaving a project restores previous PATH and values; independent terminals keep their own selection.

## Review the project

Open the checked-in project roots: `project-a`, `project-b`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu install
oyzu exec -- node probe.mjs
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

This is an interaction/evidence contract. Review its input files and expected outcomes; no substitute Oyzu implementation is supplied.

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **nested-terminal:** Open both directories in separate activated terminals. Expected: Neither changes the other's tool or APP_MODE.

## Contract and limitations

Acceptance criteria: ENV-01, ENV-03. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
